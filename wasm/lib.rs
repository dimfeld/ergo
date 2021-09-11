use ergo_tasks::{TaskConfig, ValidateError};
use fxhash::FxHashSet;
use itertools::Itertools;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(getter_with_clone)]
#[derive(Clone, Default)]
pub struct Location {
    pub path: Option<String>,
    pub line: Option<i32>,
    pub column: Option<i32>,
}

#[wasm_bindgen]
#[derive(Clone)]
pub enum ErrorType {
    Json,
    State,
}

#[wasm_bindgen(getter_with_clone)]
pub struct LintResult {
    pub message: String,
    #[wasm_bindgen(js_name = "type")]
    pub error_type: ErrorType,
    pub expected: Option<String>,
    pub location: Option<Location>,
}

#[wasm_bindgen]
pub struct LintResults(Vec<LintResult>);

#[wasm_bindgen]
pub struct Validator {
    actions: FxHashSet<String>,
    inputs: FxHashSet<String>,
}

#[wasm_bindgen]
impl Validator {
    #[wasm_bindgen(constructor)]
    pub fn new(actions: &js_sys::Set, inputs: &js_sys::Set) -> Self {
        let actions = actions
            .values()
            .into_iter()
            .filter_map(|value| value.unwrap().as_string())
            .collect::<FxHashSet<_>>();
        let inputs = inputs
            .values()
            .into_iter()
            .filter_map(|value| value.unwrap().as_string())
            .collect::<FxHashSet<_>>();

        Validator { actions, inputs }
    }

    pub fn validate_config(&self, content: JsValue) -> Result<LintResults, JsValue> {
        let config = serde_wasm_bindgen::from_value::<TaskConfig>(content)?;

        let errs = config
            .validate(&self.actions, &self.inputs)
            .err()
            .map(|e| e.0)
            .unwrap_or_else(Vec::new)
            .into_iter()
            .map(|e| {
                let error_type = match e {
                    ValidateError::InvalidInitialState(_) => ErrorType::State,
                    ValidateError::InvalidTriggerId { .. } => ErrorType::State,
                };

                LintResult {
                    message: e.to_string(),
                    error_type,
                    expected: e.expected().clone().map(|e| e.into_owned()),
                    location: e.path().map(|p| Location {
                        path: Some(p.into_iter().join(".")),
                        ..Default::default()
                    }),
                }
            })
            .collect::<Vec<_>>();

        Ok(LintResults(errs))
    }
}
