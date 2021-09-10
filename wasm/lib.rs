use ergo_tasks::{state_machine::StateMachineConfig, TaskConfig};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone)]
pub struct Location {
    pub line: i32,
    pub column: i32,
}

#[wasm_bindgen]
#[derive(Clone)]
pub enum ErrorType {
    Json,
    InvalidInitialState,
}

#[wasm_bindgen(getter_with_clone)]
pub struct LintResult {
    pub message: String,
    #[wasm_bindgen(js_name = "type")]
    pub error_type: ErrorType,
    pub invalid_value: Option<String>,
    pub location: Option<Location>,
}

#[wasm_bindgen]
pub struct LintResults(Vec<LintResult>);

#[wasm_bindgen]
pub fn lint_task_config(actions: &js_sys::Set, inputs: &js_sys::Set, content: &str) -> LintResults {
    let parse_result = json5::from_str::<TaskConfig>(content);

    let errors = match parse_result {
        Ok(TaskConfig::StateMachine(m)) => lint_state_machines(actions, inputs, &m),
        Err(json5::Error::Message { msg, location }) => vec![LintResult {
            message: msg,
            error_type: ErrorType::Json,
            invalid_value: None,
            location: location.map(|l| Location {
                line: l.line as i32,
                column: l.column as i32,
            }),
        }],
    };

    // serde_wasm_bindgen::to_value(&errors).map_err(|e| e.into())
    LintResults(errors)
}

fn lint_state_machines(
    actions: &js_sys::Set,
    inputs: &js_sys::Set,
    m: &StateMachineConfig,
) -> Vec<LintResult> {
    let mut errors = vec![];

    for machine in m {
        if !machine.states.contains_key(&machine.initial) {
            errors.push(LintResult {
                message: format!("Invalid initial state {}", machine.initial),
                error_type: ErrorType::InvalidInitialState,
                invalid_value: Some(machine.initial.clone()),
                location: None,
            })
        }
    }

    errors
}
