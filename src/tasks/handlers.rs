use super::{
    actions::{self, queue::ActionQueue},
    inputs::{self, queue::InputQueue},
};
use crate::{
    auth,
    database::{PostgresPool, VaultPostgresPool, VaultPostgresPoolOptions},
    error::Error,
    queues::postgres_drain,
    vault::VaultClientTokenData,
};

use actix_identity::Identity;
use actix_web::{
    post, web,
    web::{Data, Path},
    HttpRequest, HttpResponse, Responder,
};
use postgres_drain::QueueStageDrain;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TaskAndTriggerPath {
    task_id: String,
    trigger_id: i64,
}

#[post("/tasks/{task_id}/trigger/{trigger_id}")]
async fn post_task_trigger(
    path: Path<TaskAndTriggerPath>,
    data: BackendAppStateData,
    req: HttpRequest,
    payload: web::Json<serde_json::Value>,
    identity: Identity,
) -> Result<impl Responder, Error> {
    let user = auth::authenticate(&data.pg, &identity, &req).await?;
    let (org_id, user_id) = user.org_and_user();

    let trigger = sqlx::query!(
        r##"SELECT tasks.*, task_trigger_id, input_id,
            inputs.payload_schema as input_schema
        FROM task_triggers tt
        JOIN user_entity_permissions p ON user_entity_id = $1
            AND permission_type = 'trigger_event'
            AND permissioned_object IN(1, task_trigger_id)
        JOIN tasks USING(task_id)
        JOIN inputs USING(input_id)
        WHERE org_id = $2 AND task_trigger_id = $3 AND external_task_id = $4
        "##,
        user_id,
        org_id,
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

pub struct BackendAppState {
    pub pg: PostgresPool,
    action_queue: actions::queue::ActionQueue,
    input_queue: inputs::queue::InputQueue,
}

pub type BackendAppStateData = Data<BackendAppState>;

pub fn app_data(
    pg_pool: VaultPostgresPool,
    input_queue: InputQueue,
    action_queue: ActionQueue,
) -> Result<BackendAppStateData, Error> {
    Ok(Data::new(BackendAppState {
        pg: pg_pool,
        action_queue,
        input_queue,
    }))
}

pub fn scope(app_data: &BackendAppStateData, root: &str) -> actix_web::Scope {
    web::scope(root)
        .app_data(app_data.clone())
        .service(post_task_trigger)
}
