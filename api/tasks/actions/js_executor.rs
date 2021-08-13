use crate::database::PostgresPool;

use super::{
    super::scripting,
    execute::{Executor, ExecutorError},
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};

use async_trait::async_trait;
use fxhash::FxHashMap;
use tracing::instrument;

const FIELD_SCRIPT: TemplateField = TemplateField::from_static(
    "script",
    TemplateFieldFormat::String,
    false,
    "The script to execute",
);
const FIELD_ARGS: TemplateField = TemplateField::from_static(
    "args",
    TemplateFieldFormat::Object,
    true,
    "Arguments to the script. Exposed as 'args' in the script",
);

#[derive(Debug)]
pub struct JsExecutor {
    template_fields: TemplateFields,
}

impl JsExecutor {
    pub fn new() -> JsExecutor {
        let template_fields = vec![FIELD_SCRIPT, FIELD_ARGS]
            .into_iter()
            .map(|val| (val.name.to_string(), val))
            .collect::<TemplateFields>();

        JsExecutor { template_fields }
    }
}

#[async_trait]
impl Executor for JsExecutor {
    fn name(&self) -> &'static str {
        "js"
    }

    #[instrument(level = "debug", name = "JsExecutor::execute", skip(_pg_pool))]
    async fn execute(
        &self,
        _pg_pool: PostgresPool,
        payload: FxHashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, ExecutorError> {
        let script = payload
            .get("script")
            .and_then(|u| u.as_str())
            .ok_or_else(|| ExecutorError::FieldFormatError {
                field: "script".to_string(),
                subfield: None,
                expected: "Javascript string".to_string(),
            });

        let mut runtime = scripting::create_executor_runtime();
        if let Some(args) = FIELD_ARGS.extract_object(&payload)? {
            runtime
                .set_global_value("args", args)
                .map_err(|e| ExecutorError::CommandError {
                    source: e,
                    result: serde_json::Value::Null,
                })?;
        } else {
            runtime
                .set_global_value("args", &serde_json::json!({}))
                .map_err(|e| ExecutorError::CommandError {
                    source: e,
                    result: serde_json::Value::Null,
                })?;
        }

        let console = runtime.take_console_messages();

        Ok(serde_json::json!({}))
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}
