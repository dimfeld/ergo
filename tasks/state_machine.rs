use ergo_database::object_id::TaskId;
pub use ergo_task_types::state_machine::*;
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

pub type StateMachineStates = SmallVec<[StateMachineData; 2]>;

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateMachineData {
    pub state: String,
    pub context: serde_json::Value,
}

async fn resolve_actions(
    handler: &EventHandler,
    task_id: &TaskId,
    input_arrival_id: &Option<uuid::Uuid>,
    context: &serde_json::Value,
    payload: &Option<&serde_json::Value>,
) -> Result<ActionInvocations, StateMachineError> {
    match &handler.actions {
        None => Ok(ActionInvocations::new()),
        Some(actions) => {
            let mut output = ActionInvocations::with_capacity(actions.len());
            for def in actions {
                let built_payload = build_action(&def.data, context, payload).await?;
                event!(Level::DEBUG, ?context, ?built_payload, "built payload");
                let invocation = ActionInvocation {
                    input_arrival_id: input_arrival_id.clone(),
                    actions_log_id: uuid::Uuid::new_v4(),
                    task_id: task_id.clone(),
                    task_action_local_id: def.task_action_local_id.clone(),
                    payload: built_payload,
                };
                output.push(invocation);
            }
            Ok(output)
        }
    }
}

async fn resolve_next_state(
    handler: &EventHandler,
    context: &serde_json::Value,
    payload: &Option<&serde_json::Value>,
) -> Result<Option<String>, StateMachineError> {
    match &handler.target {
        None => Ok(None),
        Some(TransitionTarget::One(s)) => Ok(Some(s.clone())),
        Some(TransitionTarget::Script(s)) => {
            scripting::run_simple_with_context_and_payload(s.as_str(), Some(context), *payload)
                .await
                .map_err(StateMachineError::ScriptError)
        }
    }
}

async fn build_action(
    builder: &ActionPayloadBuilder,
    context: &serde_json::Value,
    payload: &Option<&serde_json::Value>,
) -> Result<serde_json::Value, StateMachineError> {
    match builder {
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
            let result =
                scripting::run_simple_with_context_and_payload(s.as_str(), Some(context), *payload)
                    .await
                    .map_err(StateMachineError::ScriptError);

            result
        }
    }
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
                let next_state = resolve_next_state(&h, &self.data.context, &payload).await?;
                let actions = resolve_actions(
                    h,
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
