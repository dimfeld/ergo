use super::{actions, inputs};
use crate::{
    auth,
    database::{PostgresPool, VaultPostgresPool, VaultPostgresPoolOptions},
    error::Error,
    queues::postgres_drain,
    service_config::Config,
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
        &payload.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Accepted().finish())
}

pub struct BackendAppState {
    pg: PostgresPool,
    action_queue: actions::queue::ActionQueue,
    input_queue: inputs::queue::InputQueue,
}

pub type BackendAppStateData = Data<BackendAppState>;

pub fn app_data(config: Config<impl VaultClientTokenData>) -> Result<BackendAppStateData, Error> {
    let pg_pool = VaultPostgresPool::new(VaultPostgresPoolOptions {
        max_connections: 16,
        host: config.database_host,
        database: config.database.unwrap_or_else(|| "ergo".to_string()),
        auth: config.database_auth,
        shutdown: config.shutdown.clone(),
    })?;

    let redis_pool = deadpool_redis::Config {
        url: Some(config.redis_host),
        pool: None,
    }
    .create_pool()?;

    Ok(Data::new(BackendAppState {
        pg: pg_pool,
        action_queue: actions::queue::new(redis_pool.clone()),
        input_queue: inputs::queue::new(redis_pool.clone()),
    }))
}

pub fn scope(app_data: &BackendAppStateData, root: &str) -> actix_web::Scope {
    web::scope(root)
        .app_data(app_data.clone())
        .service(post_task_trigger)
}
