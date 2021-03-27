use crate::error::Error;
use crate::vault::VaultPostgresPool;
use actix_web::{
    get, post, web,
    web::{Data, Path},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TaskAndTriggerPath {
    task_id: String,
    trigger_id: String,
}

#[post("/tasks/{task_id}/trigger/{trigger_id}")]
async fn post_task_trigger(
    path: Path<TaskAndTriggerPath>,
    data: BackendAppStateData,
) -> Result<impl Responder, Error> {
    Ok("")
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
