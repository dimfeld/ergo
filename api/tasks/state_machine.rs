use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;
use tracing::{event, instrument, Level};

use super::actions::ActionInvocation;

#[derive(Debug, Error)]
pub enum StateMachineError {
    #[error("Machine {idx} unknown state {state}")]
    UnknownState { idx: usize, state: String },
    #[error("Context is missing required field {0}")]
    ContextMissingField(String),
    #[error("Payload is missing required field {0}")]
    InputPayloadMissingField(String),
}

pub type StateMachineConfig = SmallVec<[StateMachine; 2]>;
pub type StateMachineStates = SmallVec<[StateMachineData; 2]>;

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateMachineData {
    pub state: String,
    pub context: serde_json::Value,
}

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

impl EventHandler {
    fn resolve_actions(
        &self,
        task_id: i64,
        input_arrival_id: &Option<uuid::Uuid>,
        context: &serde_json::Value,
        payload: &Option<&serde_json::Value>,
    ) -> Result<ActionInvocations, StateMachineError> {
        self.actions
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|def| {
                let payload = def.data.build(context, payload)?;

                Ok(ActionInvocation {
                    input_arrival_id: input_arrival_id.clone(),
                    actions_log_id: uuid::Uuid::new_v4(),
                    task_id,
                    task_action_local_id: def.task_action_local_id.clone(),
                    payload,
                })
            })
            .collect::<Result<ActionInvocations, StateMachineError>>()
    }

    fn next_state(
        &self,
        _context: &serde_json::Value,
        _payload: &Option<&serde_json::Value>,
    ) -> Result<Option<String>, StateMachineError> {
        match &self.target {
            None => Ok(None),
            Some(TransitionTarget::One(s)) => Ok(Some(s.clone())),
        }
    }
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "t", content = "c")]
pub enum TransitionTarget {
    One(String),
    // Cond(Vec<TransitionCondition>),
    // Script(String),
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransitionCondition {
    pub target: String,
    pub cond: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "t", content = "c")]
pub enum ActionPayloadBuilder {
    FieldMap(FxHashMap<String, ActionInvokeDefDataField>),
    // Script(String),
}

impl ActionPayloadBuilder {
    fn build(
        &self,
        context: &serde_json::Value,
        payload: &Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, StateMachineError> {
        match self {
            ActionPayloadBuilder::FieldMap(data) => data
                .iter()
                .map(|(key, invoke_def)| {
                    let value: Result<serde_json::Value, StateMachineError> = match &invoke_def {
                        ActionInvokeDefDataField::Constant(v) => Ok(v.clone()),
                        ActionInvokeDefDataField::Input(path, required) => {
                            let payload_value = payload.as_ref().and_then(|p| p.pointer(path));
                            match (payload_value, *required) {
                                (None, true) => {
                                    Err(StateMachineError::InputPayloadMissingField(path.clone()))
                                }
                                (None, false) => Ok(serde_json::Value::Null),
                                (Some(v), _) => Ok(v.clone()),
                            }
                        }
                        ActionInvokeDefDataField::Context(path, required) => {
                            let context_value = context.pointer(path);
                            match (context_value, *required) {
                                (None, true) => {
                                    Err(StateMachineError::ContextMissingField(path.clone()))
                                }
                                (None, false) => Ok(serde_json::Value::Null),
                                (Some(v), _) => Ok(v.clone()),
                            }
                        }
                    };

                    value.map(|v| (key.clone(), v))
                })
                .collect::<Result<serde_json::Map<String, serde_json::Value>, StateMachineError>>()
                .map(serde_json::Value::Object),
        }
    }
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionInvokeDef {
    pub task_action_local_id: String,
    pub data: ActionPayloadBuilder,
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
    // /// A script that calculates a value
    // Script(String),
}

pub type ActionInvocations = SmallVec<[ActionInvocation; 4]>;

#[derive(Debug)]
pub struct StateMachineWithData {
    task_id: i64,
    idx: usize,
    machine: StateMachine,
    data: StateMachineData,
    changed: bool,
}

impl<'d> StateMachineWithData {
    pub fn new(
        task_id: i64,
        idx: usize,
        machine: StateMachine,
        data: StateMachineData,
    ) -> StateMachineWithData {
        StateMachineWithData {
            task_id,
            idx,
            machine,
            data,
            changed: false,
        }
    }

    pub fn take(self) -> (StateMachineData, bool) {
        (self.data, self.changed)
    }

    #[instrument(fields(actions))]
    pub fn apply_trigger(
        &mut self,
        trigger_id: &str,
        input_arrival_id: &Option<uuid::Uuid>,
        payload: Option<&serde_json::Value>,
    ) -> Result<ActionInvocations, StateMachineError> {
        let handler = {
            self.machine
                .states
                .get(&self.data.state)
                .ok_or_else(|| StateMachineError::UnknownState {
                    idx: self.idx,
                    state: self.data.state.clone(),
                })?
                .on
                .iter()
                .find(|o| o.trigger_id == trigger_id)
                .or_else(|| {
                    // Look it up in the global event handlers
                    self.machine.on.iter().find(|o| o.trigger_id == trigger_id)
                })
        };

        match handler {
            Some(h) => {
                event!(Level::DEBUG, handler=?h, "Running event handler");
                let next_state = h.next_state(&self.data.context, &payload)?;
                let actions = h.resolve_actions(
                    self.task_id,
                    input_arrival_id,
                    &self.data.context,
                    &payload,
                )?;

                if let Some(s) = next_state {
                    if self.data.state != s {
                        self.changed = true;
                        self.data.state = s;
                    }
                }

                Ok(actions)
            }
            None => {
                event!(Level::DEBUG, "No handler");
                Ok(ActionInvocations::new())
            }
        }
    }
}