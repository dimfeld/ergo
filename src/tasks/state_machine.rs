use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateMachineError {
    #[error("Machine {idx} unknown state {state}")]
    UnknownState { idx: usize, state: String },
}

pub type StateMachineConfig = SmallVec<[StateMachine; 2]>;
pub type StateMachineStates = SmallVec<[StateMachineData; 2]>;

#[derive(Debug, Serialize, Deserialize)]
pub struct StateMachineData {
    state: String,
    context: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateMachine {
    name: String,
    description: Option<String>,
    initial: String,
    states: FxHashMap<String, StateDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateDefinition {
    description: String,
    on: SmallVec<[EventHandler; 4]>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventHandler {
    trigger_id: i64,
    target: Option<TransitionTarget>,
    actions: Option<Vec<ActionInvokeDef>>,
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
pub struct ActionInvokeDef {
    action_id: i64,
    data: FxHashMap<String, ActionInvokeDefDataField>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum ActionInvokeDefDataField {
    Context(String),
    Event(String),
    Script(String),
}

#[derive(Debug, Serialize)]
pub struct ActionInvocation {
    task_id: i64,
    task_trigger_id: Option<i64>,
    action_id: i64,
    payload: serde_json::Value,
}

pub type ActionInvocations = SmallVec<[ActionInvocation; 4]>;

pub struct StateMachineWithData {
    idx: usize,
    machine: StateMachine,
    data: StateMachineData,
}

impl StateMachineWithData {
    pub fn new(idx: usize, machine: StateMachine, data: StateMachineData) -> StateMachineWithData {
        StateMachineWithData { idx, machine, data }
    }

    pub fn apply_trigger(
        &mut self,
        trigger_id: i64,
        payload: Option<serde_json::Value>,
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
        };

        match handler {
            Some(h) => {
                let next_state = h.next_state(&self.data.context, &payload)?;
                let actions = h.resolve_actions(&self.data.context, &payload)?;

                if let Some(s) = next_state {
                    self.data.state = s;
                }

                Ok(actions)
            }
            None => Ok(ActionInvocations::new()),
        }
    }

    fn resolve_actions(
        &self,
        handler: &EventHandler,
        payload: Option<serde_json::Value>,
    ) -> Result<ActionInvocations, StateMachineError> {
        Ok(SmallVec::new())
    }
}

impl EventHandler {
    fn resolve_actions(
        &self,
        context: &serde_json::Value,
        payload: &Option<serde_json::Value>,
    ) -> Result<ActionInvocations, StateMachineError> {
        Ok(ActionInvocations::new())
    }

    fn next_state(
        &self,
        context: &serde_json::Value,
        payload: &Option<serde_json::Value>,
    ) -> Result<Option<String>, StateMachineError> {
        Ok(None)
    }
}
