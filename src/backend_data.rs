use actix_web::web::Data;

use crate::{
    auth::AuthData,
    database::PostgresPool,
    error::Result,
    tasks::{actions::queue::ActionQueue, inputs::queue::InputQueue},
};

pub struct BackendAppState {
    pub pg: PostgresPool,
    pub auth: AuthData,
    action_queue: ActionQueue,
    input_queue: InputQueue,
}

pub type BackendAppStateData = Data<BackendAppState>;

pub fn app_data(
    pg_pool: PostgresPool,
    input_queue: InputQueue,
    action_queue: ActionQueue,
) -> Result<BackendAppStateData> {
    Ok(Data::new(BackendAppState {
        auth: AuthData::new(pg_pool.clone())?,
        pg: pg_pool,
        action_queue,
        input_queue,
    }))
}