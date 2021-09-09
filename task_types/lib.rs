use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod scripting;
pub mod state_machine;

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum TaskConfig {
    StateMachine(state_machine::StateMachineConfig),
    // JS(scripting::TaskJsConfig),
}
