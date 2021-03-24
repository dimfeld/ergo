use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};

pub type StateMachineConfig = FxHashMap<String, StateMachine>;
pub type StateMachineStates = FxHashMap<String, StateMachineData>;

#[derive(Debug, Serialize, Deserialize)]
pub struct StateMachineData {
    state: String,
    context: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateMachine {
    description: String,
    initial: String,
    states: FxHashMap<String, StateDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateDefinition {
    description: String,
    on: FxHashMap<String, EventHandler>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventHandler {
    target: Option<TransitionTarget>,
    actions: Option<Vec<ActionInvocation>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum TransitionTarget {
    One(String),
    Cond(Vec<TransitionCondition>),
    Script(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransitionCondition {
    target: String,
    condition: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionInvocation {
    action_id: i64,
    data: FxHashMap<String, ActionInvocationDataField>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum ActionInvocationDataField {
    Context(String),
    Event(String),
    Script(String),
}
