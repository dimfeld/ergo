use chrono::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(not(target_family = "wasm"))]
mod runtime;
#[cfg(not(target_family = "wasm"))]
pub use runtime::*;
#[cfg(not(target_family = "wasm"))]
pub mod immediate;

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskJsConfig {
    pub timeout: Option<usize>,
    pub script: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, Eq, PartialEq)]
pub struct TaskJsState {
    pub context: String,
}
