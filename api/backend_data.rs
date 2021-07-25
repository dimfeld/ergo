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
    pub notifications: crate::notifications::NotificationManager,
    action_queue: ActionQueue,
    input_queue: InputQueue,
    pub immediate_inputs: bool,
    pub immediate_actions: bool,
}

pub type BackendAppStateData = Data<BackendAppState>;

pub fn app_data(
    pg_pool: PostgresPool,
    notifications: crate::notifications::NotificationManager,
    input_queue: InputQueue,
    action_queue: ActionQueue,
    immediate_inputs: bool,
    immediate_actions: bool,
) -> Result<BackendAppStateData> {
    Ok(Data::new(BackendAppState {
        auth: AuthData::new(pg_pool.clone())?,
        pg: pg_pool,
        notifications,
        action_queue,
        input_queue,
        immediate_inputs,
        immediate_actions,
    }))
}
