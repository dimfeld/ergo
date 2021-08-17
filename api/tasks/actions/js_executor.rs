use crate::{database::PostgresPool, tasks::scripting};

use super::{
    execute::{Executor, ExecutorError},
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};

use async_trait::async_trait;
use futures::future::TryFutureExt;
use fxhash::FxHashMap;
use tracing::instrument;

const FIELD_NAME: TemplateField = TemplateField::from_static(
    "name",
    TemplateFieldFormat::String,
    true,
    "The name of the action",
);
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
        let template_fields = vec![FIELD_NAME, FIELD_SCRIPT, FIELD_ARGS]
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

    async fn execute(
        &self,
        _pg_pool: PostgresPool,
        payload: FxHashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, ExecutorError> {
        let (console, result) = scripting::POOL
            .run(move || async move {
                let name = FIELD_NAME.extract_str(&payload)?.unwrap_or("script");
                let script = FIELD_SCRIPT.extract_str(&payload)?.unwrap_or("");

                let mut runtime = scripting::create_executor_runtime();
                if let Some(args) = FIELD_ARGS.extract_object(&payload)? {
                    runtime.set_global_value("args", args)?;
                } else {
                    runtime.set_global_value("args", &serde_json::json!({}))?;
                }

                // TODO Catch exceptions
                runtime.execute_script(name, script);
                runtime.run_event_loop(false).await;

                let console = runtime.take_console_messages();
                let result = runtime
                    .get_global_value::<serde_json::Value>("result")
                    .unwrap();
                Ok::<_, anyhow::Error>((console, result))
            })
            .await
            .map_err(|e| ExecutorError::CommandError {
                source: e,
                result: serde_json::Value::Null,
            })?;

        Ok(serde_json::json!({}))
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}
