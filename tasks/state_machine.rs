use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;

#[cfg(not(target_family = "wasm"))]
pub use native::*;

use crate::{
    actions::{Action, TaskAction},
    inputs::Input,
    TaskTrigger, TaskValidateError,
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
    ScriptError(ergo_js::Error),
}

pub type StateMachineConfig = SmallVec<[StateMachine; 1]>;
pub type StateMachineStates = SmallVec<[StateMachineData; 1]>;

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
    pub on: SmallVec<[EventHandler; 1]>,
    pub states: FxHashMap<String, StateDefinition>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateDefinition {
    pub description: Option<String>,
    pub on: SmallVec<[EventHandler; 2]>,
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
#[serde(tag = "t", content = "c")]
pub enum ActionPayloadBuilder {
    FieldMap(FxHashMap<String, ActionInvokeDefDataField>),
    Script(String),
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

impl StateMachine {
    pub fn validate(
        &self,
        actions: &FxHashMap<String, Action>,
        inputs: &FxHashMap<String, Input>,
        task_triggers: &FxHashMap<String, TaskTrigger>,
        task_actions: &FxHashMap<String, TaskAction>,
    ) -> Vec<TaskValidateError> {
        let mut errors = Vec::new();

        if !self.states.contains_key(&self.initial) {
            errors.push(TaskValidateError::InvalidInitialState(self.initial.clone()));
        }

        self.validate_handlers(
            actions,
            inputs,
            task_triggers,
            task_actions,
            &mut errors,
            None,
            &self.on,
        );

        for (state_name, state) in self.states.iter() {
            self.validate_handlers(
                actions,
                inputs,
                task_triggers,
                task_actions,
                &mut errors,
                Some(state_name),
                &state.on,
            );
        }

        errors
    }

    fn validate_handlers(
        &self,
        actions: &FxHashMap<String, Action>,
        inputs: &FxHashMap<String, Input>,
        task_triggers: &FxHashMap<String, TaskTrigger>,
        task_actions: &FxHashMap<String, TaskAction>,
        errors: &mut Vec<TaskValidateError>,
        state: Option<&String>,
        handlers: &[EventHandler],
    ) {
        for (index, handler) in handlers.iter().enumerate() {
            if !task_triggers.contains_key(&handler.trigger_id) {
                errors.push(TaskValidateError::InvalidTriggerId {
                    trigger_id: handler.trigger_id.clone(),
                    state: state.cloned(),
                    index,
                });
            }

            // TODO actions

            // Make sure transition target points to a valid state
            match handler.target.as_ref() {
                Some(TransitionTarget::One(s)) => {
                    if !self.states.contains_key(s) {
                        errors.push(TaskValidateError::InvalidTarget {
                            state: state.cloned(),
                            index,
                            target: s.clone(),
                        });
                    }
                }
                // TODO What can we do here?
                Some(TransitionTarget::Script(_)) => {}
                None => {}
            }
        }
    }

    pub fn default_state(&self) -> StateMachineData {
        StateMachineData {
            state: self.initial.to_string(),
            context: serde_json::json!({}),
        }
    }
}

#[cfg(not(target_family = "wasm"))]
mod native {
    use ergo_database::{
        new_uuid,
        object_id::{TaskId, UserId},
    };
    use tracing::{event, instrument, Level};

    use super::*;
    use crate::{
        actions::{ActionInvocation, ActionInvocations},
        scripting::{self, run_simple_with_context_and_payload},
    };

    #[derive(Debug)]
    pub struct StateMachineWithData {
        task_id: TaskId,
        idx: usize,
        machine: StateMachine,
        data: StateMachineData,
        changed: bool,
    }

    impl EventHandler {
        async fn resolve_actions(
            &self,
            task_id: &TaskId,
            user_id: &UserId,
            input_arrival_id: &Option<uuid::Uuid>,
            context: &serde_json::Value,
            payload: &Option<&serde_json::Value>,
        ) -> Result<ActionInvocations, StateMachineError> {
            match &self.actions {
                None => Ok(ActionInvocations::new()),
                Some(actions) => {
                    let mut output = ActionInvocations::with_capacity(actions.len());
                    for def in actions {
                        let built_payload = def.data.build(context, payload).await?;
                        event!(Level::DEBUG, ?context, ?built_payload, "built payload");
                        let invocation = ActionInvocation {
                            input_arrival_id: *input_arrival_id,
                            actions_log_id: new_uuid(),
                            task_id: task_id.clone(),
                            task_action_local_id: def.task_action_local_id.clone(),
                            user_id: user_id.clone(),
                            payload: built_payload,
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
                    scripting::run_simple_with_context_and_payload(
                        s.as_str(),
                        Some(context),
                        *payload,
                    )
                    .await
                    .map_err(StateMachineError::ScriptError)
                }
            }
        }
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
                                    (None, true) => Err(
                                        StateMachineError::InputPayloadMissingField(path.clone()),
                                    ),
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
                    let result = scripting::run_simple_with_context_and_payload(
                        s.as_str(),
                        Some(context),
                        *payload,
                    )
                    .await
                    .map_err(StateMachineError::ScriptError);

                    result
                }
            }
        }
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
            user_id: &UserId,
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
                            user_id,
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
}
