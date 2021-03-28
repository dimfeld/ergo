use crate::{
    auth::{self, Permission},
    error::Error,
    vault::VaultPostgresPool,
};
use actix_identity::Identity;
use actix_web::{
    get, post, web,
    web::{Data, Path},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use serde::Deserialize;
use sqlx::query;

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
    payload: web::Json<Box<serde_json::value::RawValue>>,
    identity: Identity,
) -> Result<impl Responder, Error> {
    let user = auth::authenticate(&data.pg, &identity, &req).await?;
    let (org_id, user_id) = user.org_and_user();

    let trigger = sqlx::query!(
        r##"SELECT task_trigger_id, task_id, input_id
        FROM task_triggers
        JOIN user_entity_permissions p ON user_entity_id = $1
            AND permission_type = 'trigger_event'
            AND permissioned_object IN(1, task_trigger_id)
        JOIN tasks USING(task_id)
        WHERE org_id = $2 AND task_trigger_id = $3
        "##,
        user_id,
        org_id,
        path.trigger_id
    )
    .fetch_optional(&data.pg)
    .await?
    .ok_or(Error::AuthorizationError)?;

    super::inputs::enqueue_input(
        trigger.task_id,
        trigger.input_id,
        trigger.task_trigger_id,
        payload.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Accepted().finish())
}

pub struct BackendAppState {
    pg: VaultPostgresPool<()>,
}

pub type BackendAppStateData = Data<BackendAppState>;

pub fn scope(app_data: &BackendAppStateData, root: &str) -> actix_web::Scope {
    web::scope(root)
        .app_data(app_data.clone())
        .service(post_task_trigger)
}
