use std::borrow::Cow;

use ergo_tasks::{TaskConfig, ValidatePathSegment};
use fxhash::FxHashSet;
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
    actions: FxHashSet<String>,
    inputs: FxHashSet<String>,
}

#[wasm_bindgen]
impl TaskConfigValidator {
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

        TaskConfigValidator { actions, inputs }
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
            .validate(&self.actions, &self.inputs)
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
                                        PathSegment::String(Cow::from(s.clone()))
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
