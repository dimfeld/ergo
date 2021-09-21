use super::{
    execute::{get_primitive_payload_value, json_primitive_as_string, Executor, ExecutorError},
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};
use anyhow::anyhow;
use async_trait::async_trait;
#[cfg(not(target_family = "wasm"))]
use ergo_database::PostgresPool;
use fxhash::FxHashMap;
use serde_json::json;
use std::process::Stdio;
use tracing::{event, instrument, Level};

#[cfg(target_family = "unix")]
use std::os::unix::process::ExitStatusExt;

const FIELD_COMMAND: TemplateField = TemplateField::from_static(
    "command",
    TemplateFieldFormat::String,
    false,
    "The executable to run",
);
const FIELD_ARGS: TemplateField = TemplateField::from_static(
    "args",
    TemplateFieldFormat::StringArray,
    true,
    "An array of arguments to the executable",
);
const FIELD_ENV: TemplateField = TemplateField::from_static(
    "env",
    TemplateFieldFormat::Object,
    true,
    "Environment variables to set",
);
const FIELD_ALLOW_FAILURE: TemplateField = TemplateField::from_static(
    "allow_failure",
    TemplateFieldFormat::Boolean,
    true,
    "If true, ignore the process exit code. By default, a nonzero exit code counts as failure",
);

#[derive(Debug)]
pub struct RawCommandExecutor {
    template_fields: TemplateFields,
}

impl RawCommandExecutor {
    pub fn new() -> RawCommandExecutor {
        let template_fields = vec![FIELD_COMMAND, FIELD_ARGS, FIELD_ENV, FIELD_ALLOW_FAILURE];

        RawCommandExecutor { template_fields }
    }
}

#[async_trait]
impl Executor for RawCommandExecutor {
    fn name(&self) -> &'static str {
        "raw_command"
    }

    #[cfg(not(target_family = "wasm"))]
    #[instrument(level = "debug", name = "RawCommandExecutor::execute", skip(_pg_pool))]
    async fn execute(
        &self,
        _pg_pool: PostgresPool,
        payload: FxHashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, ExecutorError> {
        let command = get_primitive_payload_value(&payload, "command", false)?;

        let mut cmd = tokio::process::Command::new(command.as_ref());

        // Don't leak our environment, which may contain secrets, to other commands.
        cmd.env_clear();
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

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

        let allow_failure = match payload.get("allow_failure") {
            Some(serde_json::Value::Bool(b)) => *b,
            _ => false,
        };

        event!(Level::DEBUG, ?cmd);

        let output = cmd
            .output()
            .await
            .map_err(|e| ExecutorError::CommandError {
                source: e.into(),
                result: json!(null),
            })?;

        let exitcode = output.status.code();
        event!(Level::DEBUG, exitcode = ?exitcode);

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        event!(Level::TRACE, %stdout, %stderr);

        let result = json!({
            "exitcode": exitcode,
            "stdout": stdout,
            "stderr": stderr,
        });

        if !output.status.success() && !allow_failure {
            let msg = match (exit_status_message(&output.status), exitcode) {
                (Some(m), _) => m,
                (None, Some(code)) => format!("Exited with code {}", code),
                (None, None) => "Exited with unknown error".to_string(),
            };

            return Err(ExecutorError::CommandError {
                source: anyhow!(msg),
                result,
            });
        }

        Ok(result)
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}

#[cfg(unix)]
fn exit_status_message(e: &std::process::ExitStatus) -> Option<String> {
    if let Some(signal) = e.signal() {
        return Some(format!("Exited with signal {}", signal));
    }

    None
}

#[cfg(not(unix))]
fn exit_status_message(e: &std::process::ExitStatus) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[tokio::test]
    // fn test_basic_command() {
    //     let (_, exec) = RawCommandExecutor::new();
    //     let result = exec.execute(pg_pool, args).await;
    // }
}
