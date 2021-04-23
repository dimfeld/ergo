use crate::{database::PostgresPool, error::Error};

use super::{
    execute::{get_primitive_payload_value, json_primitive_as_string, Executor, ExecutorError},
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};
use async_trait::async_trait;
use fxhash::FxHashMap;

#[derive(Debug)]
pub struct RawCommandExecutor {
    template_fields: TemplateFields,
}

impl RawCommandExecutor {
    pub fn new() -> (String, Box<dyn Executor>) {
        let template_fields = vec![
            (
                "command",
                TemplateField {
                    format: TemplateFieldFormat::String,
                    optional: false,
                    description: Some("The executable to run".to_string()),
                },
            ),
            (
                "args",
                TemplateField {
                    format: TemplateFieldFormat::StringArray,
                    optional: true,
                    description: Some("An array of arguments to the executable".to_string()),
                },
            ),
            (
                "env",
                TemplateField {
                    format: TemplateFieldFormat::Object,
                    optional: true,
                    description: Some("Environment variables to set".to_string()),
                },
            ),
        ]
        .into_iter()
        .map(|(key, val)| (key.to_string(), val))
        .collect::<TemplateFields>();

        (
            "raw_command".to_string(),
            Box::new(RawCommandExecutor { template_fields }),
        )
    }
}

#[async_trait]
impl Executor for RawCommandExecutor {
    async fn execute(
        &self,
        pg_pool: PostgresPool,
        payload: FxHashMap<String, serde_json::Value>,
    ) -> Result<(), ExecutorError> {
        let command = get_primitive_payload_value(&payload, "command", false)?;

        let mut cmd = tokio::process::Command::new(command.as_ref());

        // Don't leak our environment, which may contain secrets, to other commands.
        cmd.env_clear();

        if let Some(args) = payload.get("args") {
            match args {
                serde_json::Value::Array(array) => {
                    for v in array {
                        let value = json_primitive_as_string("args", None, v, false)?;
                        cmd.arg(value.as_ref());
                    }
                }
                _ => {}
            }
        }

        if let Some(env) = payload.get("env") {
            match env {
                serde_json::Value::Object(m) => {
                    for (k, v) in m {
                        let value = json_primitive_as_string("env", Some(k), v, false)?;
                        cmd.env(k, value.as_ref());
                    }
                }
                _ => {}
            }
        }

        cmd.spawn()
            .map_err(|e| ExecutorError::CommandError(e.into()))?
            .wait()
            .await
            .map_err(|e| ExecutorError::CommandError(e.into()))?;

        Ok(())
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}
