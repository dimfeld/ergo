use super::{
    actions::{self, queue::ActionQueue},
    inputs::{self, queue::InputQueue},
    Task,
};
use crate::{
    auth::{self, AuthData, Authenticated, MaybeAuthenticated},
    backend_data::BackendAppStateData,
    database::{PostgresPool, VaultPostgresPool, VaultPostgresPoolOptions},
    error::{Error, Result},
    queues::postgres_drain,
    vault::VaultClientTokenData,
};

use actix_identity::Identity;
use actix_web::{
    delete, get, post, put, web,
    web::{Data, Path},
    HttpRequest, HttpResponse, Responder,
};
use chrono::{DateTime, Utc};
use postgres_drain::QueueStageDrain;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct TaskAndTriggerPath {
    task_id: String,
    trigger_id: i64,
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
    data: BackendAppStateData,
    req: HttpRequest,

    auth: Authenticated,
) -> Result<impl Responder> {
    let user_ids = auth.user_entity_ids();
    let tasks = sqlx::query_as!(
        TaskDescription,
        "SELECT external_task_id AS id, name, description, enabled, created, modified
        FROM tasks
        JOIN user_entity_permissions ON
            permissioned_object = tasks.task_id
            AND user_entity_id = ANY($1)
            AND permission_type = 'read'
        WHERE tasks.org_id = $2",
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
    data: BackendAppStateData,
    req: HttpRequest,
    auth: Authenticated,
) -> Result<impl Responder> {
    Ok(HttpResponse::NotImplemented().finish())
}

#[delete("/tasks/{task_id}")]
async fn delete_task(
    task_id: Path<String>,
    data: BackendAppStateData,
    req: HttpRequest,
    auth: Authenticated,
) -> Result<impl Responder> {
    Ok(HttpResponse::NotImplemented().finish())
}

#[put("/tasks/{task_id}")]
async fn update_task(
    task_id: Path<String>,
    data: BackendAppStateData,
    req: HttpRequest,
    auth: Authenticated,
) -> Result<impl Responder> {
    Ok(HttpResponse::NotImplemented().finish())
}

#[post("/tasks")]
async fn new_task(
    req: HttpRequest,
    data: BackendAppStateData,
    auth: Authenticated,
    payload: web::Json<Task>,
) -> Result<impl Responder> {
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
        JOIN user_entity_permissions p ON user_entity_id = ANY($1)
            AND permission_type = 'trigger_event'
            AND permissioned_object IN(1, task_trigger_id)
        JOIN tasks USING(task_id)
        JOIN inputs USING(input_id)
        WHERE org_id = $2 AND task_trigger_id = $3 AND external_task_id = $4
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

pub fn scope(app_data: &BackendAppStateData, root: &str) -> actix_web::Scope {
    web::scope(root)
        .app_data(app_data.clone())
        .service(post_task_trigger)
}
