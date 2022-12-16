use std::borrow::Cow;

use super::{
    execute::{Executor, ExecutorError},
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};

#[cfg(not(target_family = "wasm"))]
use crate::scripting;
use async_trait::async_trait;

use fxhash::FxHashMap;
#[cfg(not(target_family = "wasm"))]
use tracing::{event, Level};
use url::Url;

static FIELD_NAME: TemplateField = TemplateField::from_static(
    "name",
    TemplateFieldFormat::String {
        default: Cow::Borrowed("script"),
    },
    true,
    "The name of the action",
);
static FIELD_SCRIPT: TemplateField = TemplateField::from_static(
    "script",
    TemplateFieldFormat::string_without_default(),
    false,
    "The script to execute",
);
static FIELD_ARGS: TemplateField = TemplateField::from_static(
    "args",
    TemplateFieldFormat::Object {
        nested: true,
        default: Cow::Borrowed("{}"),
    },
    true,
    "Arguments to the script. Exposed as 'args' in the script",
);

#[derive(Debug)]
pub struct JsExecutor {
    template_fields: TemplateFields,
}

impl JsExecutor {
    pub fn new() -> JsExecutor {
        let template_fields = [&FIELD_NAME, &FIELD_SCRIPT, &FIELD_ARGS].into();

        JsExecutor { template_fields }
    }
}

const EXECUTOR_STARTUP_SCRIPT: &str = r##"
globalThis.Ergo = globalThis.Ergo || {};
Ergo.setResult = function(value) {
    globalThis.__ergo_result = value;
}
"##;

#[async_trait]
impl Executor for JsExecutor {
    fn name(&self) -> &'static str {
        "js"
    }

    #[cfg(not(target_family = "wasm"))]
    async fn execute(
        &self,
        _state: super::execute::ExecutorState,
        payload: FxHashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, ExecutorError> {
        let (console, result) = scripting::POOL
            .run(move || async move {
                let name = FIELD_NAME.extract_str(&payload)?;
                let script = FIELD_SCRIPT.extract_str(&payload)?;

                let mut runtime = scripting::create_executor_runtime();
                let args = FIELD_ARGS.extract_object(&payload)?;
                runtime
                    .set_global_value("args", args.as_ref())
                    .map_err(ExecutorError::command_error_without_result)?;

                event!(Level::DEBUG, %script, "executing script");
                let name_url =
                    Url::parse(&format!("https://ergo/executor/{}", name)).map_err(|_| {
                        ExecutorError::FieldFormatError {
                            field: "name".to_string(),
                            subfield: None,
                            expected: "A formattable name".to_string(),
                        }
                    })?;

                runtime
                    .execute_script("executor_init", EXECUTOR_STARTUP_SCRIPT)
                    .map_err(ExecutorError::command_error_without_result)?;

                let run_result = runtime.run_main_module(name_url, script.to_string()).await;
                let mut console = serde_json::to_value(runtime.take_console_messages())
                    .unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));

                run_result.map_err(|e| ExecutorError::CommandError {
                    source: e.into(),
                    result: std::mem::take(&mut console),
                })?;

                let result = runtime
                    .get_global_value::<serde_json::Value>("__ergo_result")
                    .map_err(|e| ExecutorError::CommandError {
                        source: e.into(),
                        result: std::mem::take(&mut console),
                    })?
                    .unwrap_or(serde_json::Value::Null);
                Ok::<_, ExecutorError>((console, result))
            })
            .await?;

        Ok(serde_json::json!({
            "result": result,
            "console": console
        }))
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore]
    async fn runs_script() {
        let script = r##"
            
        "##;
    }

    #[tokio::test]
    #[ignore]
    async fn runs_async_script() {}

    #[tokio::test]
    #[ignore]
    async fn script_exception() {}

    #[tokio::test]
    #[ignore]
    async fn async_script_exception() {}
}
