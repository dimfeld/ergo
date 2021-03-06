use ergo_database::object_id::TaskId;
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;
use tracing::{event, instrument, Level};

use super::{
    actions::ActionInvocation,
    scripting::{self, run_simple_with_context_and_payload},
};

#[derive(Debug, Error)]
pub enum StateMachineError {
    #[error("Machine {idx} unknown state {state}")]
    UnknownState { idx: usize, state: String },
    #[error("Context is missing required field {0}")]
    ContextMissingField(String),
    #[error("Payload is missing required field {0}")]
    InputPayloadMissingField(String),
    #[error(transparent)]
    ScriptError(anyhow::Error),
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
    async fn resolve_actions(
        &self,
        task_id: &TaskId,
        input_arrival_id: &Option<uuid::Uuid>,
        context: &serde_json::Value,
        payload: &Option<&serde_json::Value>,
    ) -> Result<ActionInvocations, StateMachineError> {
        match &self.actions {
            None => Ok(ActionInvocations::new()),
            Some(actions) => {
                let mut output = ActionInvocations::with_capacity(actions.len());
                for def in actions {
                    let payload = def.data.build(context, payload).await?;
                    let invocation = ActionInvocation {
                        input_arrival_id: input_arrival_id.clone(),
                        actions_log_id: uuid::Uuid::new_v4(),
                        task_id: task_id.clone(),
                        task_action_local_id: def.task_action_local_id.clone(),
                        payload,
                    };
                    output.push(invocation);
                }
                Ok(output)
            }
        }
    }

    async fn next_state(
        &self,
        context: &serde_json::Value,
        payload: &Option<&serde_json::Value>,
    ) -> Result<Option<String>, StateMachineError> {
        match &self.target {
            None => Ok(None),
            Some(TransitionTarget::One(s)) => Ok(Some(s.clone())),
            Some(TransitionTarget::Script(s)) => {
                scripting::run_simple_with_context_and_payload(s.as_str(), Some(context), *payload)
                    .await
                    .map_err(StateMachineError::ScriptError)
            }
        }
    }
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
#[serde(tag = "t", content = "c")]
pub enum ActionPayloadBuilder {
    FieldMap(FxHashMap<String, ActionInvokeDefDataField>),
    Script(String),
}

impl ActionPayloadBuilder {
    async fn build(
        &self,
        context: &serde_json::Value,
        payload: &Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, StateMachineError> {
        match self {
            ActionPayloadBuilder::FieldMap(data) => {
                let mut output = serde_json::Map::with_capacity(data.len());
                for (key, invoke_def) in data {
                    let value: serde_json::Value = match &invoke_def {
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
                        ActionInvokeDefDataField::Script(script) => {
                            run_simple_with_context_and_payload::<serde_json::Value>(
                                script.as_str(),
                                Some(context),
                                *payload,
                            )
                            .await
                            .map_err(StateMachineError::ScriptError)
                        }
                    }?;
                    output.insert(key.clone(), value);
                }

                Ok(serde_json::Value::Object(output))
            }
            ActionPayloadBuilder::Script(s) => {
                scripting::run_simple_with_context_and_payload(s.as_str(), Some(context), *payload)
                    .await
                    .map_err(StateMachineError::ScriptError)
            }
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
    /// A script that calculates a value
    Script(String),
}

pub type ActionInvocations = SmallVec<[ActionInvocation; 4]>;

#[derive(Debug)]
pub struct StateMachineWithData {
    task_id: TaskId,
    idx: usize,
    machine: StateMachine,
    data: StateMachineData,
    changed: bool,
}

impl<'d> StateMachineWithData {
    pub fn new(
        task_id: TaskId,
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
    pub async fn apply_trigger(
        &mut self,
        trigger_id: &str,
        input_arrival_id: &Option<uuid::Uuid>,
        payload: Option<&serde_json::Value>,
    ) -> Result<ActionInvocations, StateMachineError> {
        let handler = self
            .machine
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
            });

        match handler {
            Some(h) => {
                event!(Level::DEBUG, handler=?h, "Running event handler");
                let next_state = h.next_state(&self.data.context, &payload).await?;
                let actions = h
                    .resolve_actions(
                        &self.task_id,
                        input_arrival_id,
                        &self.data.context,
                        &payload,
                    )
                    .await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[tokio::test]
    #[ignore]
    async fn trigger_changes_state() {}

    #[tokio::test]
    #[ignore]
    async fn trigger_without_handler() {}

    #[tokio::test]
    #[ignore]
    async fn global_trigger_handler() {}

    #[tokio::test]
    #[ignore]
    async fn next_state_script() {}

    #[tokio::test]
    #[ignore]
    async fn next_state_script_returns_null() {}

    #[tokio::test]
    #[ignore]
    async fn next_state_script_returns_same_state() {}
}
