use super::{
    actions::{self, queue::ActionQueue, TaskAction},
    inputs::{self, queue::InputQueue},
    state_machine, Task,
};
use crate::{
    auth::{self, AuthData, Authenticated, MaybeAuthenticated},
    backend_data::BackendAppStateData,
    database::{
        object_id::new_object_id, PostgresPool, VaultPostgresPool, VaultPostgresPoolOptions,
    },
    error::{Error, Result},
    queues::postgres_drain,
    vault::VaultClientTokenData,
    web_app_server::AppStateData,
};

use actix_identity::Identity;
use actix_web::{
    delete, get, post, put, web,
    web::{Data, Path},
    HttpRequest, HttpResponse, Responder, Scope,
};
use chrono::{DateTime, Utc};
use fxhash::FxHashMap;
use postgres_drain::QueueStageDrain;
use serde::{Deserialize, Serialize};
use sqlx::Connection;

#[derive(Debug, Deserialize)]
struct TaskAndTriggerPath {
    task_id: String,
    trigger_id: String,
}

#[derive(Debug, Serialize)]
struct TaskDescription {
    id: String,
    name: String,
    description: Option<String>,
    enabled: bool,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
}

#[get("/tasks")]
async fn list_tasks(
    data: AppStateData,
    req: HttpRequest,
    auth: Authenticated,
) -> Result<impl Responder> {
    let user_ids = auth.user_entity_ids();
    let tasks = sqlx::query_as!(
        TaskDescription,
        "SELECT external_task_id AS id, name, description, enabled, created, modified
        FROM tasks
        JOIN user_entity_permissions ON
            permissioned_object IN (1, tasks.task_id)
            AND user_entity_id = ANY($1)
            AND permission_type = 'read'
        WHERE tasks.org_id = $2 AND
            EXISTS (SELECT 1 FROM user_entity_permissions
                WHERE permissioned_object IN (1, tasks.task_id)
                AND user_entity_id = ANY($1)
                AND permission_type = 'read'
            )",
        user_ids.as_slice(),
        auth.org_id(),
    )
    .fetch_all(&data.pg)
    .await?;

    Ok(HttpResponse::Ok().json(tasks))
}

#[get("/tasks/{task_id}")]
async fn get_task(
    task_id: Path<String>,
    data: AppStateData,
    req: HttpRequest,
    auth: Authenticated,
) -> Result<impl Responder> {
    let task_id = task_id.into_inner();
    let user_ids = auth.user_entity_ids();

    #[derive(Debug, Serialize, sqlx::FromRow)]
    struct TaskResult {
        task_id: String,
        name: String,
        description: Option<String>,
        enabled: bool,
        state_machine_config: serde_json::Value,
        state_machine_states: serde_json::Value,
        created: DateTime<Utc>,
        modified: DateTime<Utc>,
        triggers: Option<serde_json::Value>,
        actions: Option<serde_json::Value>,
    }

    let task = sqlx::query_as!(
        TaskResult,
        r##"SELECT external_task_id as task_id,
        tasks.name, tasks.description, enabled,
        state_machine_config, state_machine_states,
        created, modified,
        task_triggers as triggers,
        task_actions as actions
        FROM tasks

        LEFT JOIN LATERAL (
            SELECT jsonb_object_agg(task_action_local_id, jsonb_build_object(
                'action_id', action_id,
                'account_id', account_id,
                'name', task_actions.name,
                'action_template', task_actions.action_template
            )) AS task_actions

            FROM task_actions WHERE task_actions.task_id = tasks.task_id
            GROUP BY task_actions.task_id
        ) ta ON true

        LEFT JOIN LATERAL (
            SELECT jsonb_object_agg(task_trigger_local_id, jsonb_build_object(
                'input_id', input_id,
                'name', task_triggers.name,
                'description', task_triggers.description
            )) task_triggers
            FROM task_triggers WHERE task_triggers.task_id = tasks.task_id
            GROUP BY task_triggers.task_id
        ) tt ON true

        WHERE external_task_id=$1 AND org_id=$3
        AND EXISTS(SELECT 1 FROM user_entity_permissions
            WHERE
            permissioned_object IN (1, task_id)
            AND user_entity_id=ANY($2)
            AND permission_type = 'read'
        )"##,
        &task_id,
        user_ids.as_slice(),
        auth.org_id()
    )
    .fetch_optional(&data.pg)
    .await?;

    match task {
        Some(task) => Ok(HttpResponse::Ok().json(task)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

#[delete("/tasks/{task_id}")]
async fn delete_task(
    task_id: Path<String>,
    data: AppStateData,
    req: HttpRequest,
    auth: Authenticated,
) -> Result<impl Responder> {
    Ok(HttpResponse::NotImplemented().finish())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskActionInput {
    pub task_local_id: String,
    pub name: String,
    pub action_id: i64,
    pub account_id: Option<i64>,
    pub action_template: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskTriggerInput {
    pub input_id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskInput {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub state_machine_config: state_machine::StateMachineConfig,
    pub state_machine_states: state_machine::StateMachineStates,
    pub actions: FxHashMap<String, TaskActionInput>,
    pub triggers: FxHashMap<String, TaskTriggerInput>,
}

#[put("/tasks/{task_id}")]
async fn update_task(
    task_id: Path<String>,
    data: AppStateData,
    req: HttpRequest,
    auth: Authenticated,
    payload: web::Json<TaskInput>,
) -> Result<impl Responder> {
    Ok(HttpResponse::NotImplemented().finish())
}

#[post("/tasks")]
async fn new_task(
    req: HttpRequest,
    data: AppStateData,
    auth: Authenticated,
    payload: web::Json<TaskInput>,
) -> Result<impl Responder> {
    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    let external_task_id =
        base64::encode_config(uuid::Uuid::new_v4().as_bytes(), base64::URL_SAFE_NO_PAD);

    let task_id = new_object_id(&mut tx).await?;
    sqlx::query!(
        "INSERT INTO tasks (task_id, external_task_id, org_id, name,
        description, enabled, state_machine_config, state_machine_states) VALUES
        ($1, $2, $3, $4, $5, $6, $7, $8)",
        task_id,
        external_task_id,
        auth.org_id(),
        payload.name,
        payload.description,
        payload.enabled,
        sqlx::types::Json(payload.state_machine_config.as_slice()) as _,
        sqlx::types::Json(payload.state_machine_states.as_slice()) as _
    )
    .execute(&mut tx)
    .await?;

    for (local_id, action) in &payload.actions {
        sqlx::query!(
            "INSERT INTO task_actions (task_id, task_action_local_id,
                action_id, account_id, name, action_template)
                VALUES
                ($1, $2, $3, $4, $5, $6)",
            task_id,
            local_id,
            action.action_id,
            action.account_id,
            action.name,
            sqlx::types::Json(action.action_template.as_ref()) as _
        )
        .execute(&mut tx)
        .await?;
    }

    for (local_id, trigger) in &payload.triggers {
        let trigger_id = new_object_id(&mut tx).await?;
        sqlx::query!(
            "INSERT INTO task_triggers (task_trigger_id, task_id, input_id, task_trigger_local_id,
                name, description
            ) VALUES
            ($1, $2, $3, $4, $5, $6)",
            trigger_id,
            task_id,
            trigger.input_id,
            local_id,
            trigger.name,
            trigger.description as _
        )
        .execute(&mut tx)
        .await?;
    }

    tx.commit().await?;

    Ok(HttpResponse::NotImplemented().finish())
}

#[post("/tasks/{task_id}/trigger/{trigger_id}")]
async fn post_task_trigger(
    path: Path<TaskAndTriggerPath>,
    data: BackendAppStateData,
    req: HttpRequest,
    auth: Authenticated,
    payload: web::Json<serde_json::Value>,
) -> Result<impl Responder> {
    let ids = auth.user_entity_ids();

    let trigger = sqlx::query!(
        r##"SELECT tasks.*, task_trigger_id, input_id,
            inputs.payload_schema as input_schema
        FROM task_triggers tt
        JOIN tasks USING(task_id)
        JOIN inputs USING(input_id)
        WHERE org_id = $2 AND task_trigger_local_id = $3 AND external_task_id = $4
            AND EXISTS(
                SELECT 1 FROM user_entity_permissions
                WHERE user_entity_id = ANY($1)
                AND permission_type = 'trigger_event'
                AND permissioned_object IN(1, task_trigger_id)
            )
        "##,
        ids.as_slice(),
        auth.org_id(),
        path.trigger_id,
        path.task_id
    )
    .fetch_optional(&data.pg)
    .await?
    .ok_or(Error::AuthorizationError)?;

    super::inputs::enqueue_input(
        &data.pg,
        trigger.task_id,
        trigger.input_id,
        trigger.task_trigger_id,
        &trigger.input_schema,
        payload.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Accepted().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(post_task_trigger)
        .service(list_tasks)
        .service(new_task)
        .service(update_task)
        .service(delete_task)
        .service(super::inputs::handlers::list_inputs)
        .service(super::inputs::handlers::new_input)
        .service(super::inputs::handlers::write_input)
        .service(super::inputs::handlers::delete_input)
        .service(super::actions::handlers::list_actions)
        .service(super::actions::handlers::new_action)
        .service(super::actions::handlers::write_action)
        .service(super::actions::handlers::delete_action);
}
