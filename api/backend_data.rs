use actix_web::web::Data;
use ergo_auth::AuthData;
use ergo_database::PostgresPool;
use ergo_notifications::NotificationManager;
use ergo_tasks::{actions::queue::ActionQueue, inputs::queue::InputQueue};

use crate::error::Result;

pub struct BackendAppState {
    pub pg: PostgresPool,
    pub auth: AuthData,
    pub notifications: NotificationManager,
    action_queue: ActionQueue,
    input_queue: InputQueue,
    pub redis_key_prefix: Option<String>,
}

pub type BackendAppStateData = Data<BackendAppState>;

pub fn app_data(
    pg_pool: PostgresPool,
    notifications: NotificationManager,
    input_queue: InputQueue,
    action_queue: ActionQueue,
    redis_key_prefix: Option<String>,
) -> Result<BackendAppStateData> {
    Ok(Data::new(BackendAppState {
        auth: AuthData::new(pg_pool.clone())?,
        pg: pg_pool,
        notifications,
        action_queue,
        input_queue,
        redis_key_prefix,
    }))
}
