use crate::{
    backend_data::BackendAppStateData,
    error::{Error, Result},
    web_app_server::AppStateData,
};

use actix_web::{
    delete, get, post, put,
    web::{self, Path},
    HttpRequest, HttpResponse, Responder,
};
use chrono::{DateTime, Utc};
use ergo_auth::Authenticated;
use ergo_database::object_id::{
    AccountId, ActionId, InputId, OrgId, TaskId, TaskTemplateId, TaskTriggerId, UserId,
};
use ergo_tasks::{
    actions::{ActionStatus, TaskAction, TaskActionTemplate},
    inputs::{EnqueueInputOptions, InputStatus},
    PeriodicTaskTriggerInput, TaskConfig, TaskState, TaskTrigger,
};
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{Connection, Postgres, Transaction};
use std::str::FromStr;
use tracing::{field, instrument};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct TaskAndTriggerPath {
    task_id: String,
    trigger_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TaskDescription {
    pub task_id: TaskId,
    pub name: String,
    pub description: Option<String>,
    pub alias: Option<String>,
    pub enabled: bool,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
    pub successes: i64,
    pub failures: i64,
    pub stats_since: DateTime<Utc>,
}

#[get("/tasks")]
async fn list_tasks(data: AppStateData, auth: Authenticated) -> Result<impl Responder> {
    let user_ids = auth.user_entity_ids();
    let tasks = sqlx::query_as!(
        TaskDescription,
        r##"SELECT task_id AS "task_id: TaskId", name, description, alias, enabled, created, modified,
            last_triggered AS "last_triggered?",
            COALESCE(successes, 0) as "successes!",
            COALESCE(failures, 0) as "failures!",
            (now() - '7 days'::interval) as "stats_since!"
        FROM tasks
        LEFT JOIN LATERAL(
            SELECT
                SUM(CASE WHEN al.status = 'success' OR (al.status IS NULL AND il.status = 'success') THEN 1 ELSE 0 END) AS successes,
                SUM(CASE WHEN al.status = 'error' OR (al.status IS NULL AND il.status = 'error') THEN 1 ELSE 0 END) AS failures
            FROM inputs_log il
            LEFT JOIN actions_log al USING(inputs_log_id)
            WHERE il.task_id = tasks.task_id AND il.updated > (now() - '7 days'::interval)
        ) stat_counts ON true
        LEFT JOIN LATERAL (
            SELECT created AS last_triggered
            FROM inputs_log
            WHERE inputs_log.task_id = tasks.task_id
            ORDER BY created DESC
            LIMIT 1
        ) last_triggered ON true
        WHERE tasks.org_id = $2 AND NOT tasks.deleted AND
            EXISTS (SELECT 1 FROM user_entity_permissions
                WHERE permissioned_object IN (uuid_nil(), tasks.task_id)
                AND user_entity_id = ANY($1)
                AND permission_type = 'read'
            )"##,
        user_ids.as_slice(),
        &auth.org_id().0,
    )
    .fetch_all(&data.pg)
    .await?;

    Ok(HttpResponse::Ok().json(tasks))
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, sqlx::FromRow)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub name: String,
    pub description: Option<String>,
    pub alias: Option<String>,
    pub enabled: bool,
    pub task_template_version: i64,
    pub compiled: sqlx::types::Json<TaskConfig>,
    pub source: sqlx::types::Json<serde_json::Value>,
    pub state: sqlx::types::Json<TaskState>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub actions: sqlx::types::Json<FxHashMap<String, TaskAction>>,
    pub triggers: sqlx::types::Json<FxHashMap<String, TaskTrigger>>,
}

#[get("/tasks/{task_id}")]
#[instrument(skip(data), fields(task))]
async fn get_task(
    task_id: Path<TaskId>,
    data: AppStateData,
    req: HttpRequest,
    auth: Authenticated,
) -> Result<impl Responder> {
    let task_id = task_id.into_inner();
    let user_ids = auth.user_entity_ids();

    let task = sqlx::query_as!(
        TaskResult,
        r##"SELECT task_id as "task_id: TaskId",
        tasks.name, tasks.description, alias, enabled,
        task_template_version,
        compiled as "compiled!: _",
        source as "source!: _",
        state as "state!: _",
        tasks.created, tasks.modified,
        COALESCE(task_triggers, '{}'::jsonb) as "triggers!: _",
        COALESCE(task_actions, '{}'::jsonb) as "actions!: _"
        FROM tasks
        JOIN task_templates USING (task_template_id, task_template_version)

        LEFT JOIN LATERAL (
            SELECT jsonb_object_agg(task_action_local_id, jsonb_build_object(
                'action_id', action_id,
                'task_local_id', task_action_local_id,
                'task_id', task_actions.task_id,
                'account_id', account_id,
                'name', task_actions.name,
                'action_template', task_actions.action_template
            )) AS task_actions

            FROM task_actions WHERE task_actions.task_id = tasks.task_id
            GROUP BY task_actions.task_id
        ) ta ON true

        LEFT JOIN LATERAL (
            SELECT jsonb_object_agg(task_trigger_local_id, jsonb_build_object(
                'task_trigger_id', task_triggers.task_trigger_id,
                'task_id', task_triggers.task_id,
                'input_id', input_id,
                'name', task_triggers.name,
                'description', task_triggers.description,
                'periodic', periodic
            )) task_triggers
            FROM task_triggers
            LEFT JOIN LATERAL (
                SELECT jsonb_agg(jsonb_build_object(
                    'periodic_trigger_id', pt.periodic_trigger_id,
                    'name', pt.name,
                    'schedule', pt.schedule,
                    'payload', pt.payload,
                    'enabled', pt.enabled
                )) periodic
                FROM periodic_triggers pt WHERE pt.task_trigger_id = task_triggers.task_trigger_id
            ) AS periodic ON true
            WHERE task_triggers.task_id = tasks.task_id
            GROUP BY task_triggers.task_id
        ) tt ON true

        WHERE task_id=$1 AND tasks.org_id=$3 AND NOT tasks.deleted
        AND EXISTS(SELECT 1 FROM user_entity_permissions
            WHERE
            permissioned_object IN (uuid_nil(), task_id)
            AND user_entity_id=ANY($2)
            AND permission_type = 'read'
        )"##,
        &task_id.0,
        user_ids.as_slice(),
        &auth.org_id().0
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
    task_id: Path<TaskId>,
    data: AppStateData,
    auth: Authenticated,
) -> Result<impl Responder> {
    let user_entity_ids = auth.user_entity_ids();
    let task_id = task_id.into_inner();

    let deleted = sqlx::query_scalar!("UPDATE tasks SET deleted=true WHERE task_id=$1 AND org_id=$2
        AND NOT deleted AND
        EXISTS(SELECT 1 FROM user_entity_permissions
            WHERE permissioned_object IN (uuid_nil(), task_id) AND user_entity_id=ANY($3) AND permission_type='write'
        )
        RETURNING task_id",
        task_id.0,
        auth.org_id().0,
        user_entity_ids.as_slice())
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
    pub action_id: ActionId,
    pub account_id: Option<AccountId>,
    pub action_template: Option<TaskActionTemplate>,
}

impl PartialEq<TaskAction> for TaskActionInput {
    fn eq(&self, other: &TaskAction) -> bool {
        self.name == other.name
            && self.action_id == other.action_id
            && self.account_id == other.account_id
            && self.action_template == other.action_template
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
pub struct TaskTriggerInput {
    pub input_id: InputId,
    pub name: String,
    pub description: Option<String>,
    pub periodic: Option<Vec<PeriodicTaskTriggerInput>>,
}

impl PartialEq<TaskTrigger> for TaskTriggerInput {
    fn eq(&self, other: &TaskTrigger) -> bool {
        self.input_id == other.input_id
            && self.name == other.name
            && self.description == other.description
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct TaskInput {
    pub name: String,
    pub description: Option<String>,
    pub alias: Option<String>,
    pub enabled: bool,
    pub compiled: TaskConfig,
    pub source: serde_json::Value,
    pub state: Option<TaskState>,
    pub actions: FxHashMap<String, TaskActionInput>,
    pub triggers: FxHashMap<String, TaskTriggerInput>,
}

#[put("/tasks/{task_id}")]
async fn update_task(
    task_id: Path<TaskId>,
    data: AppStateData,
    auth: Authenticated,
    payload: web::Json<TaskInput>,
) -> Result<HttpResponse> {
    let user_ids = auth.user_entity_ids();
    let task_id = task_id.into_inner();
    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    // TODO Validate task actions against action templates.

    struct TaskUpdateResult {
        task_template_id: Uuid,
        task_template_version: i64,
    }

    let TaskUpdateResult {
        task_template_id,
        task_template_version,
    } = sqlx::query_as!(
        TaskUpdateResult,
        "UPDATE tasks SET
        name=$2, description=$3, alias=$4, enabled=$5,
        state=COALESCE($6, state),
        modified=now()
        WHERE task_id=$1 AND org_id=$7 AND EXISTS (
            SELECT 1 FROM user_entity_permissions
            WHERE permissioned_object IN (uuid_nil(), tasks.task_id)
            AND user_entity_id=ANY($8)
            AND permission_type = 'write'
            )
        RETURNING task_template_id, task_template_version
        ",
        task_id.0,
        payload.name,
        payload.description as _,
        payload.alias as _,
        payload.enabled,
        payload.state.as_ref().map(sqlx::types::Json) as _,
        auth.org_id().0,
        user_ids.as_slice()
    )
    .fetch_optional(&mut tx)
    .await?
    .ok_or(Error::NotFound)?;

    sqlx::query!(
        "UPDATE task_templates
        SET source=$3, compiled=$4
        WHERE task_template_id=$1 AND task_template_version=$2",
        task_template_id,
        task_template_version,
        &payload.source,
        sqlx::types::Json(&payload.compiled) as _,
    )
    .execute(&mut tx)
    .await?;

    for (action_local_id, action) in &payload.actions {
        sqlx::query!(
            "INSERT INTO task_actions
            (task_id, task_action_local_id, action_id, account_id, name, action_template)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (task_id, task_action_local_id) DO UPDATE SET
                action_id=EXCLUDED.action_id, account_id=EXCLUDED.account_id,
                name=EXCLUDED.name, action_template=EXCLUDED.action_template",
            &task_id.0,
            action_local_id,
            &action.action_id.0,
            action.account_id.as_ref().map(|x| x.0),
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
            &task_id.0,
            action_local_ids.as_slice() as _
        )
        .execute(&mut tx)
        .await?;
    }

    let user_id = auth.user_id();
    let org_id = auth.org_id();
    for (trigger_local_id, trigger) in &payload.triggers {
        let updated = sqlx::query!(
            "UPDATE task_triggers
            SET input_id=$3, name=$4, description=$5
            WHERE task_id=$1 and task_trigger_local_id=$2
            RETURNING task_trigger_id",
            &task_id.0,
            &trigger_local_id,
            &trigger.input_id.0,
            &trigger.name,
            &trigger.description as _
        )
        .fetch_optional(&mut tx)
        .await?;

        let trigger_id = match updated {
            Some(r) => TaskTriggerId::from_uuid(r.task_trigger_id),
            None => {
                // The object didn't exist, so update it here.
                add_task_trigger(
                    &mut tx,
                    &data.redis_key_prefix,
                    trigger_local_id,
                    &task_id,
                    &payload.name,
                    payload.enabled,
                    trigger,
                    user_id,
                    org_id,
                )
                .await?
            }
        };

        let empty_vec = Vec::new();
        let periodic = trigger.periodic.as_ref().unwrap_or(&empty_vec);
        ergo_tasks::periodic::update_triggers(
            &mut tx,
            data.redis_key_prefix.as_deref(),
            &trigger.input_id,
            &trigger_id,
            trigger_local_id.as_str(),
            &trigger.name,
            &task_id,
            &payload.name,
            payload.enabled,
            user_id,
            org_id,
            periodic.as_slice(),
        )
        .await?;
    }

    let task_trigger_ids = payload
        .triggers
        .keys()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();
    if !task_trigger_ids.is_empty() {
        sqlx::query!(
            "DELETE FROM task_triggers WHERE task_id=$1 AND task_trigger_local_id <> ALL($2)",
            &task_id.0,
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
    redis_key_prefix: &Option<String>,
    local_id: &str,
    task_id: &TaskId,
    task_name: &str,
    task_enabled: bool,
    trigger: &TaskTriggerInput,
    user_id: &UserId,
    org_id: &OrgId,
) -> Result<TaskTriggerId> {
    let trigger_id = TaskTriggerId::new();
    sqlx::query!(
        "INSERT INTO task_triggers (task_trigger_id, task_id, input_id, task_trigger_local_id,
                name, description
            ) VALUES
            ($1, $2, $3, $4, $5, $6)",
        trigger_id.0,
        task_id.0,
        trigger.input_id.0,
        local_id,
        trigger.name,
        trigger.description as _
    )
    .execute(&mut *tx)
    .await?;

    if let Some(periodic) = trigger.periodic.as_ref() {
        ergo_tasks::periodic::update_triggers(
            tx,
            redis_key_prefix.as_deref(),
            &trigger.input_id,
            &trigger_id,
            local_id,
            &trigger.name,
            task_id,
            task_name,
            task_enabled,
            user_id,
            org_id,
            periodic.as_slice(),
        )
        .await?;
    }

    sqlx::query!(
        "INSERT INTO user_entity_permissions (user_entity_id, permission_type, permissioned_object)
        VALUES ($1, 'trigger_event', $2)",
        user_id.0,
        trigger_id.0
    )
    .execute(&mut *tx)
    .await?;

    Ok(trigger_id)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewTaskResult {
    pub task_id: TaskId,
}

#[post("/tasks")]
async fn new_task_handler(
    data: AppStateData,
    auth: Authenticated,
    payload: web::Json<TaskInput>,
) -> Result<HttpResponse> {
    new_task(data, auth, payload).await
}

#[instrument(skip(data))]
async fn new_task(
    data: AppStateData,
    auth: Authenticated,
    payload: web::Json<TaskInput>,
) -> Result<HttpResponse> {
    let payload = payload.into_inner();
    let user_id = auth.user_id();

    // TODO Validate task actions against action templates.

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    let task_id = TaskId::new();
    let task_template_id = TaskTemplateId::new();
    let org_id = auth.org_id();

    let task_state = payload
        .state
        .unwrap_or_else(|| payload.compiled.default_state());

    sqlx::query!(
        r##"
        INSERT INTO task_templates (task_template_id, task_template_version, org_id,
            name, description, source, compiled, initial_state) VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8)"##,
        &task_template_id.0,
        0,
        &org_id.0,
        &payload.name,
        payload.description,
        &payload.source,
        sqlx::types::Json(payload.compiled) as _,
        sqlx::types::Json(&task_state) as _
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!(
        "INSERT INTO tasks (task_id, org_id, task_template_id, task_template_version, name,
        description, alias, enabled, state) VALUES
        ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        &task_id.0,
        &org_id.0,
        &task_template_id.0,
        0,
        payload.name,
        payload.description,
        payload.alias,
        payload.enabled,
        sqlx::types::Json(&task_state) as _
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!(
        "INSERT INTO user_entity_permissions (user_entity_id, permission_type, permissioned_object)
            VALUES
            ($1, 'read', $2),
            ($1, 'write', $2)",
        &auth.user_id().0,
        &task_id.0
    )
    .execute(&mut tx)
    .await?;

    for (local_id, action) in &payload.actions {
        sqlx::query!(
            "INSERT INTO task_actions (task_id, task_action_local_id,
                action_id, account_id, name, action_template)
                VALUES
                ($1, $2, $3, $4, $5, $6)",
            &task_id.0,
            local_id,
            &action.action_id.0,
            action.account_id.as_ref().map(|x| x.0),
            action.name,
            sqlx::types::Json(action.action_template.as_ref()) as _
        )
        .execute(&mut tx)
        .await?;
    }

    for (local_id, trigger) in &payload.triggers {
        add_task_trigger(
            &mut tx,
            &data.redis_key_prefix,
            local_id,
            &task_id,
            payload.name.as_str(),
            payload.enabled,
            trigger,
            user_id,
            org_id,
        )
        .await?;
    }

    tx.commit().await?;

    Ok(HttpResponse::Created().json(NewTaskResult { task_id }))
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskTriggerResponse {
    pub log_id: Uuid,
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

    let (task_query_field, task_query_value, task_query_field_cast) = TaskId::from_str(&task_id)
        .map(|s| ("task_id", s.as_uuid().to_string(), "uuid"))
        .unwrap_or(("alias", task_id, "text"));

    tracing::event!(tracing::Level::INFO, %task_query_field, %task_query_value);

    #[derive(sqlx::FromRow)]
    struct QueryResult {
        task_id: TaskId,
        task_name: String,
        task_trigger_name: String,
        task_trigger_id: TaskTriggerId,
        input_id: InputId,
        input_schema: serde_json::Value,
    }

    let trigger: QueryResult = sqlx::query_as(&format!(
        r##"SELECT tasks.task_id,
            tasks.name as task_name,
            tt.name as task_trigger_name,
            task_trigger_id,
            input_id,
            inputs.payload_schema as input_schema
        FROM task_triggers tt
        JOIN tasks USING(task_id)
        JOIN inputs USING(input_id)
        WHERE org_id = $2 AND task_trigger_local_id = $3 AND {} = $4::{}
            AND EXISTS(
                SELECT 1 FROM user_entity_permissions
                WHERE user_entity_id = ANY($1)
                AND permission_type = 'trigger_event'
                AND permissioned_object IN(uuid_nil(), task_trigger_id)
            )
        "##,
        task_query_field, task_query_field_cast
    ))
    .bind(ids.as_slice())
    .bind(org_id.0)
    .bind(&trigger_id)
    .bind(task_query_value)
    .fetch_optional(&data.pg)
    .await?
    .ok_or(Error::NotFound)?;

    let mut conn = data.pg.acquire().await?;
    let input_arrival_id = ergo_tasks::inputs::enqueue_input(EnqueueInputOptions {
        pg: &mut conn,
        notifications: Some(data.notifications.clone()),
        org_id: org_id.clone(),
        task_id: trigger.task_id,
        input_id: trigger.input_id,
        task_trigger_id: trigger.task_trigger_id,
        task_trigger_local_id: trigger_id,
        task_trigger_name: trigger.task_trigger_name,
        task_name: trigger.task_name,
        user_id: auth.user_id().clone(),
        payload_schema: &trigger.input_schema,
        payload: payload.into_inner(),
        redis_key_prefix: data.redis_key_prefix.as_deref(),
        trigger_at: None,
        periodic_trigger_id: None,
    })
    .await?;

    Ok(HttpResponse::Accepted().json(TaskTriggerResponse {
        log_id: input_arrival_id,
    }))
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct InputLogEntryAction {
    pub actions_log_id: Uuid,
    pub task_action_local_id: String,
    pub task_action_name: String,
    pub result: serde_json::Value,
    pub status: ActionStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct InputsLogEntry {
    pub inputs_log_id: Uuid,
    pub task_name: String,
    pub task_id: TaskId,
    pub input_status: InputStatus,
    pub info: serde_json::Value,
    pub task_trigger_name: String,
    pub task_trigger_local_id: String,
    pub timestamp: DateTime<Utc>,
    pub actions: sqlx::types::Json<Vec<InputLogEntryAction>>,
}

#[get("/logs")]
async fn get_logs(data: BackendAppStateData, auth: Authenticated) -> Result<impl Responder> {
    let ids = auth.user_entity_ids();
    let org_id = auth.org_id();

    let logs = sqlx::query_as!(
        InputsLogEntry,
        r##"
            SELECT inputs_log_id,
                tasks.name AS task_name,
                tasks.task_id AS "task_id: TaskId",
                il.status AS "input_status!: InputStatus",
                COALESCE(il.info, 'null'::jsonb) AS "info!",
                MAX(tt.name) AS "task_trigger_name!",
                il.task_trigger_local_id,
                il.updated AS "timestamp",
                COALESCE(
                    jsonb_agg(jsonb_build_object(
                        'actions_log_id', al.actions_log_id,
                        'task_action_local_id', ta.task_action_local_id,
                        'task_action_name', ta.name,
                        'result', COALESCE(al.result, 'null'::jsonb),
                        'status', al.status,
                        'timestamp', al.updated
                    ))
                    FILTER (WHERE al.actions_log_id IS NOT NULL)
                , '[]'::jsonb) AS "actions!: sqlx::types::Json<Vec<InputLogEntryAction>>"
            FROM tasks
            JOIN inputs_log il USING (task_id)
            LEFT JOIN actions_log al USING(inputs_log_id)
            LEFT JOIN task_actions ta USING(task_action_local_id)
            JOIN task_triggers tt USING(task_trigger_id)
            WHERE tasks.org_id = $2 AND
                EXISTS(SELECT 1 FROM user_entity_permissions
                    WHERE user_entity_id = ANY($1)
                    AND permission_type = 'read'
                    AND permissioned_object IN (uuid_nil(), tasks.task_id)
                )
            GROUP BY tasks.task_id, inputs_log_id
            ORDER BY il.updated DESC
            LIMIT 50
        "##,
        ids.as_slice(),
        org_id.0
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
        .service(get_logs);
}
