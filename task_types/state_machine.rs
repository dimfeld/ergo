use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
pub type StateMachineConfig = SmallVec<[StateMachine; 2]>;

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateMachine {
    pub name: String,
    pub description: Option<String>,
    pub initial: String,
    #[serde(default)]
    pub on: SmallVec<[EventHandler; 4]>,
    pub states: FxHashMap<String, StateDefinition>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateDefinition {
    pub description: Option<String>,
    pub on: SmallVec<[EventHandler; 4]>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventHandler {
    pub trigger_id: String,
    pub target: Option<TransitionTarget>,
    pub actions: Option<Vec<ActionInvokeDef>>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "t", content = "c")]
pub enum TransitionTarget {
    One(String),
    // Cond(Vec<TransitionCondition>),
    Script(String),
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransitionCondition {
    pub target: String,
    pub cond: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionInvokeDef {
    pub task_action_local_id: String,
    pub data: ActionPayloadBuilder,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "t", content = "c")]
pub enum ActionPayloadBuilder {
    FieldMap(FxHashMap<String, ActionInvokeDefDataField>),
    Script(String),
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "t", content = "c")]
pub enum ActionInvokeDefDataField {
    /// A path from the input that triggered the action, and whether or not it's required.
    Input(String, bool),
    /// A path from the state machine's context, and whether or not it's required.
    Context(String, bool),
    /// A constant value
    Constant(serde_json::Value),
    /// A script that calculates a value
    Script(String),
}
