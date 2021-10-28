use std::borrow::Cow;

use ergo_database::object_id::TaskTriggerId;
use ergo_tasks::{
    actions::{Action, TaskAction},
    inputs::Input,
    TaskConfig, TaskTrigger, ValidatePathSegment,
};
use fxhash::FxHashMap;
use serde::Serialize;
use serde_path_to_error::Segment;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum PathSegment<'a> {
    String(Cow<'a, str>),
    Index(usize),
}

#[derive(Serialize)]
pub struct LintResult<'a> {
    pub message: String,
    pub key: bool,
    pub path: Vec<PathSegment<'a>>,
    pub severity: LintSeverity,
}

#[wasm_bindgen]
pub struct TaskConfigValidator {
    actions: FxHashMap<String, Action>,
    inputs: FxHashMap<String, Input>,
    task_triggers: FxHashMap<String, TaskTrigger>,
    task_actions: FxHashMap<String, TaskAction>,
}

#[wasm_bindgen]
impl TaskConfigValidator {
    #[wasm_bindgen(constructor)]
    pub fn new(
        actions: JsValue,
        inputs: JsValue,
        task_triggers: JsValue,
        task_actions: JsValue,
    ) -> Result<TaskConfigValidator, JsValue> {
        let actions_de = serde_wasm_bindgen::Deserializer::from(actions);
        let actions = serde_path_to_error::deserialize(actions_de)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let inputs_de = serde_wasm_bindgen::Deserializer::from(inputs);
        let inputs = serde_path_to_error::deserialize(inputs_de)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let task_triggers_de = serde_wasm_bindgen::Deserializer::from(task_triggers);
        let task_triggers = serde_path_to_error::deserialize(task_triggers_de)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let task_actions_de = serde_wasm_bindgen::Deserializer::from(task_actions);
        let task_actions = serde_path_to_error::deserialize(task_actions_de)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(TaskConfigValidator {
            actions,
            inputs,
            task_triggers,
            task_actions,
        })
    }

    pub fn validate_config(&self, content: JsValue) -> Result<JsValue, JsValue> {
        let de = serde_wasm_bindgen::Deserializer::from(content);
        let config: TaskConfig = match serde_path_to_error::deserialize(de) {
            Ok(c) => c,
            Err(e) => {
                let path = e
                    .path()
                    .iter()
                    .map(|seg| match seg {
                        Segment::Seq { index } => PathSegment::Index(*index),
                        Segment::Map { key } => PathSegment::String(Cow::from(key.as_str())),
                        Segment::Enum { variant } => {
                            PathSegment::String(Cow::from(variant.as_str()))
                        }
                        _ => PathSegment::String(Cow::from("<unknown>")),
                    })
                    .collect::<Vec<_>>();

                return serde_wasm_bindgen::to_value(&vec![LintResult {
                    key: false,
                    path,
                    severity: LintSeverity::Error,
                    message: e.to_string(),
                }])
                .map_err(|e| e.into());
            }
        };

        let errs = config
            .validate(
                &self.actions,
                &self.inputs,
                &self.task_triggers,
                &self.task_actions,
            )
            .err()
            .map(|e| e.0)
            .unwrap_or_else(Vec::new)
            .iter()
            .map(|e| {
                let message = match e.expected() {
                    Some(ex) => format!("{}\nExpected {}", e.to_string(), ex),
                    None => e.to_string(),
                };

                LintResult {
                    message,
                    severity: LintSeverity::Error,
                    path: e
                        .path()
                        .as_ref()
                        .map(|p| {
                            p.as_inner()
                                .iter()
                                .map(|s| match s {
                                    ValidatePathSegment::Index(i) => PathSegment::Index(*i),
                                    ValidatePathSegment::String(s) => {
                                        PathSegment::String(s.clone())
                                    }
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_else(Vec::new),
                    key: false,
                }
            })
            .collect::<Vec<_>>();

        serde_wasm_bindgen::to_value(&errs).map_err(|e| e.into())
    }
}

#[wasm_bindgen]
pub fn new_task_trigger_id() -> String {
    TaskTriggerId::new().to_string()
}
