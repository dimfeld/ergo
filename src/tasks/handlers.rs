use super::{
    actions::ActionStatus,
    state_machine::{self, StateMachineConfig, StateMachineStates},
};
use crate::{
    auth::Authenticated,
    backend_data::BackendAppStateData,
    database::object_id::new_object_id,
    error::{Error, Result},
    tasks::inputs::EnqueueInputOptions,
    web_app_server::AppStateData,
};

use actix_web::{
    delete, get, post, put,
    web::{self, Path, Query},
    HttpRequest, HttpResponse, Responder,
};
use chrono::{DateTime, Utc};
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Connection, Postgres, Transaction};
use tracing::{field, instrument};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct TaskAndTriggerPath {
    task_id: String,
    trigger_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TaskDescription {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
    pub successes: i64,
    pub failures: i64,
    pub stats_since: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct TaskId {
    task_id: String,
}

#[get("/tasks")]
async fn list_tasks(data: AppStateData, auth: Authenticated) -> Result<impl Responder> {
    let user_ids = auth.user_entity_ids();
    let tasks = sqlx::query_as!(
        TaskDescription,
        r##"SELECT external_task_id AS id, name, description, enabled, created, modified,
            last_triggered AS "last_triggered?",
            COALESCE(successes, 0) as "successes!",
            COALESCE(failures, 0) as "failures!",
            (now() - '7 days'::interval) as "stats_since!"
        FROM tasks
        LEFT JOIN LATERAL(
            SELECT
                SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) AS successes,
                SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) AS failures
            FROM inputs_log
            WHERE inputs_log.task_id = tasks.task_id AND updated > (now() - '7 days'::interval)
        ) stat_counts ON true
        LEFT JOIN LATERAL (
            SELECT created AS last_triggered
            FROM inputs_log
            WHERE inputs_log.task_id = tasks.task_id
            ORDER BY created DESC
            LIMIT 1
        ) last_triggered ON true
        WHERE tasks.org_id = $2 AND NOT deleted AND
            EXISTS (SELECT 1 FROM user_entity_permissions
                WHERE permissioned_object IN (1, tasks.task_id)
                AND user_entity_id = ANY($1)
                AND permission_type = 'read'
            )"##,
        user_ids.as_slice(),
        auth.org_id(),
    )
    .fetch_all(&data.pg)
    .await?;

    Ok(HttpResponse::Ok().json(tasks))
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct TaskResult {
    pub task_id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub state_machine_config: sqlx::types::Json<StateMachineConfig>,
    pub state_machine_states: sqlx::types::Json<StateMachineStates>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub actions: sqlx::types::Json<FxHashMap<String, TaskActionInput>>,
    pub triggers: sqlx::types::Json<FxHashMap<String, TaskTriggerInput>>,
}

#[get("/tasks/{task_id}")]
#[instrument(skip(data), fields(task))]
async fn get_task(
    task_id: Path<String>,
    data: AppStateData,
    req: HttpRequest,
    auth: Authenticated,
) -> Result<impl Responder> {
    let task_id = task_id.into_inner();
    let user_ids = auth.user_entity_ids();

    let task = sqlx::query_as!(
        TaskResult,
        r##"SELECT external_task_id as task_id,
        tasks.name, tasks.description, enabled,
        state_machine_config as "state_machine_config!: _",
        state_machine_states as "state_machine_states!: _",
        created, modified,
        COALESCE(task_triggers, '{}'::jsonb) as "triggers!: _",
        COALESCE(task_actions, '{}'::jsonb) as "actions!: _"
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

        WHERE external_task_id=$1 AND org_id=$3 AND NOT deleted
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

    tracing::Span::current().record("task", &field::debug(&task));

    match task {
        Some(task) => Ok(HttpResponse::Ok().json(task)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

#[delete("/tasks/{task_id}")]
async fn delete_task(
    task_id: Path<String>,
    data: AppStateData,
    auth: Authenticated,
) -> Result<impl Responder> {
    let user_entity_ids = auth.user_entity_ids();
    let task_id = task_id.into_inner();

    let deleted = sqlx::query_scalar!("UPDATE tasks SET deleted=true WHERE external_task_id=$1 AND org_id=$2
        AND NOT deleted AND
        EXISTS(SELECT 1 FROM user_entity_permissions
            WHERE permissioned_object IN (1, task_id) AND user_entity_id=ANY($3) AND permission_type='write'
        )
        RETURNING task_id",
        task_id, auth.org_id(), user_entity_ids.as_slice())
        .fetch_optional(&data.pg)
        .await?;

    match deleted {
        Some(_) => Ok(HttpResponse::Ok().finish()),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct TaskActionInput {
    pub name: String,
    pub action_id: i64,
    pub account_id: Option<i64>,
    pub action_template: Option<Vec<(String, serde_json::Value)>>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct TaskTriggerInput {
    pub input_id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
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
    external_task_id: Path<String>,
    data: AppStateData,
    auth: Authenticated,
    payload: web::Json<TaskInput>,
) -> Result<HttpResponse> {
    let user_ids = auth.user_entity_ids();
    let external_task_id = external_task_id.into_inner();
    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    // TODO Validate task actions against action templates.

    let task_id = sqlx::query!(
        "UPDATE TASKS SET
        name=$2, description=$3, enabled=$4, state_machine_config=$5, state_machine_states=$6, modified=now()
        WHERE external_task_id=$1 AND org_id=$7 AND EXISTS (
            SELECT 1 FROM user_entity_permissions
            WHERE permissioned_object IN (1, tasks.task_id)
            AND user_entity_id=ANY($8)
            AND permission_type = 'write'
            )
        RETURNING task_id
        ",
        &external_task_id,
        &payload.name,
        &payload.description as _,
        payload.enabled,
        sqlx::types::Json(&payload.state_machine_config) as _,
        sqlx::types::Json(&payload.state_machine_states) as _,
        auth.org_id(),
        user_ids.as_slice()
    )
    .fetch_optional(&mut tx)
    .await?;

    let task_id = match task_id {
        Some(t) => t.task_id,
        None => {
            drop(tx);
            drop(conn);
            return new_task(data, auth, Some(external_task_id), payload).await;
        }
    };

    for (action_local_id, action) in &payload.actions {
        sqlx::query!(
            "INSERT INTO task_actions
            (task_id, task_action_local_id, action_id, account_id, name, action_template)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (task_id, task_action_local_id) DO UPDATE SET
                action_id=EXCLUDED.action_id, account_id=EXCLUDED.account_id,
                name=EXCLUDED.name, action_template=EXCLUDED.action_template",
            task_id,
            action_local_id,
            action.action_id,
            action.account_id,
            action.name,
            sqlx::types::Json(&action.action_template) as _
        )
        .execute(&mut tx)
        .await?;
    }

    let action_local_ids = payload
        .actions
        .keys()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();
    if !action_local_ids.is_empty() {
        sqlx::query!(
            "DELETE FROM task_actions WHERE task_id=$1 AND task_action_local_id <> ALL($2)",
            task_id,
            action_local_ids.as_slice() as _
        )
        .execute(&mut tx)
        .await?;
    }

    let user_id = auth.user_id();
    for (trigger_local_id, trigger) in &payload.triggers {
        let updated = sqlx::query!(
            "UPDATE task_triggers
            SET input_id=$3, name=$4, description=$5
            WHERE task_id=$1 and task_trigger_local_id=$2",
            task_id,
            &trigger_local_id,
            trigger.input_id,
            &trigger.name,
            &trigger.description as _
        )
        .execute(&mut tx)
        .await?;

        if updated.rows_affected() == 0 {
            // The object didn't exist, so update it here.
            add_task_trigger(&mut tx, &trigger_local_id, task_id, trigger, &user_id).await?;
        }
    }

    let task_trigger_ids = payload
        .triggers
        .keys()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();
    if !task_trigger_ids.is_empty() {
        sqlx::query!(
            "DELETE FROM task_triggers WHERE task_id=$1 AND task_triggeR_local_id <> ALL($2)",
            task_id,
            &task_trigger_ids as _
        )
        .execute(&mut tx)
        .await?;
    }

    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

async fn add_task_trigger(
    tx: &mut Transaction<'_, Postgres>,
    local_id: &str,
    task_id: i64,
    trigger: &TaskTriggerInput,
    user_id: &Option<&Uuid>,
) -> Result<i64> {
    let trigger_id = new_object_id(&mut *tx, "task_trigger").await?;
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
    .execute(&mut *tx)
    .await?;

    if let Some(user_id) = user_id {
        sqlx::query!("INSERT INTO user_entity_permissions (user_entity_id, permission_type, permissioned_object)
        VALUES ($1, 'trigger_event', $2)",
            user_id,
            trigger_id
        ).execute(&mut *tx).await?;
    }

    Ok(trigger_id)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewTaskResult {
    pub task_id: String,
}

#[post("/tasks")]
async fn new_task_handler(
    data: AppStateData,
    auth: Authenticated,
    payload: web::Json<TaskInput>,
) -> Result<HttpResponse> {
    new_task(data, auth, None, payload).await
}

#[instrument(skip(data))]
async fn new_task(
    data: AppStateData,
    auth: Authenticated,
    external_task_id: Option<String>,
    payload: web::Json<TaskInput>,
) -> Result<HttpResponse> {
    let payload = payload.into_inner();
    let external_task_id = external_task_id.unwrap_or_else(|| {
        base64::encode_config(uuid::Uuid::new_v4().as_bytes(), base64::URL_SAFE_NO_PAD)
    });
    let user_id = auth.user_id();

    // TODO Validate task actions against action templates.

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    let task_id = new_object_id(&mut tx, "task").await?;
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

    if let Some(user_id) = auth.user_id() {
        sqlx::query!(
            "INSERT INTO user_entity_permissions (user_entity_id, permission_type, permissioned_object)
            VALUES
            ($1, 'read', $2),
            ($1, 'write', $2)",
            user_id, &task_id
        )
        .execute(&mut tx)
        .await?;
    }

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
        add_task_trigger(&mut tx, &local_id, task_id, trigger, &user_id).await?;
    }

    tx.commit().await?;

    Ok(HttpResponse::Created().json(NewTaskResult {
        task_id: external_task_id,
    }))
}

#[post("/tasks/{task_id}/trigger/{trigger_id}")]
async fn post_task_trigger(
    path: Path<TaskAndTriggerPath>,
    data: BackendAppStateData,
    auth: Authenticated,
    payload: web::Json<serde_json::Value>,
) -> Result<impl Responder> {
    let ids = auth.user_entity_ids();
    let org_id = auth.org_id();

    let TaskAndTriggerPath {
        task_id,
        trigger_id,
    } = path.into_inner();

    let trigger = sqlx::query!(
        r##"SELECT tasks.*, tt.name as task_trigger_name, task_trigger_id, input_id,
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
        org_id,
        &trigger_id,
        &task_id
    )
    .fetch_optional(&data.pg)
    .await?
    .ok_or(Error::NotFound)?;

    let input_arrival_id = super::inputs::enqueue_input(EnqueueInputOptions {
        run_immediately: data.immediate_inputs,
        immediate_actions: data.immediate_actions,
        pg: &data.pg,
        notifications: Some(data.notifications.clone()),
        org_id: org_id.clone(),
        task_id: trigger.task_id,
        input_id: trigger.input_id,
        task_trigger_id: trigger.task_trigger_id,
        task_trigger_local_id: trigger_id,
        task_trigger_name: trigger.task_trigger_name,
        task_name: trigger.name,
        payload_schema: &trigger.input_schema,
        payload: payload.into_inner(),
    })
    .await?;

    Ok(HttpResponse::Accepted().json(json!({ "log_id": input_arrival_id })))
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ActionLogEntry {
    pub actions_log_id: Uuid,
    pub task_name: String,
    pub external_task_id: String,
    pub task_action_local_id: String,
    pub task_action_name: String,
    pub task_trigger_local_id: String,
    pub task_trigger_name: String,
    pub payload: serde_json::Value,
    pub result: serde_json::Value,
    pub status: ActionStatus,
    pub updated: DateTime<Utc>,
}

#[get("/logs")]
async fn get_logs(data: BackendAppStateData, auth: Authenticated) -> Result<impl Responder> {
    let ids = auth.user_entity_ids();
    let org_id = auth.org_id();

    let logs = sqlx::query_as!(
        ActionLogEntry,
        r##"
            SELECT actions_log_id,
                tasks.name AS task_name,
                tasks.external_task_id,
                ta.task_action_local_id,
                ta.name AS task_action_name,
                tt.task_trigger_local_id,
                tt.name AS task_trigger_name,
                COALESCE(al.payload, 'null'::jsonb) AS "payload!",
                COALESCE(al.result, 'null'::jsonb) AS "result!",
                al.status AS "status: ActionStatus",
                al.updated
            FROM tasks
            JOIN actions_log al USING(task_id)
            JOIN task_actions ta USING(task_action_local_id)
            JOIN inputs_log il USING(inputs_log_id)
            JOIN task_triggers tt USING(task_trigger_id)
            WHERE tasks.org_id = $2 AND
                EXISTS(SELECT 1 FROM user_entity_permissions
                    WHERE user_entity_id = ANY($1)
                    AND permission_type = 'read'
                    AND permissioned_object IN (1, tasks.task_id)
                )
            ORDER BY al.updated DESC
            LIMIT 50
        "##,
        ids.as_slice(),
        org_id
    )
    .fetch_all(&data.pg)
    .await?;

    Ok(HttpResponse::Ok().json(logs))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(post_task_trigger)
        .service(list_tasks)
        .service(get_task)
        .service(new_task_handler)
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
