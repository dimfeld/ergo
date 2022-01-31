use super::{
    execute::{get_primitive_payload_value, json_primitive_as_string, Executor, ExecutorError},
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};
use anyhow::anyhow;
use async_trait::async_trait;
use fxhash::FxHashMap;
use serde_json::json;
use std::process::Stdio;
use tracing::{event, instrument, Level};

#[cfg(target_family = "unix")]
use std::os::unix::process::ExitStatusExt;

static FIELD_COMMAND: TemplateField = TemplateField::from_static(
    "command",
    TemplateFieldFormat::string_without_default(),
    false,
    "The executable to run",
);
static FIELD_ARGS: TemplateField = TemplateField::from_static(
    "args",
    TemplateFieldFormat::string_array_without_default(),
    true,
    "An array of arguments to the executable",
);
static FIELD_ENV: TemplateField = TemplateField::from_static(
    "env",
    TemplateFieldFormat::object_without_default(false),
    true,
    "Environment variables to set",
);
static FIELD_ALLOW_FAILURE: TemplateField = TemplateField::from_static(
    "allow_failure",
    TemplateFieldFormat::Boolean { default: false },
    true,
    "If true, ignore the process exit code. By default, a nonzero exit code counts as failure",
);

#[derive(Debug)]
pub struct RawCommandExecutor {
    template_fields: TemplateFields,
}

impl RawCommandExecutor {
    pub fn new() -> RawCommandExecutor {
        let template_fields = [
            &FIELD_COMMAND,
            &FIELD_ARGS,
            &FIELD_ENV,
            &FIELD_ALLOW_FAILURE,
        ]
        .into();

        RawCommandExecutor { template_fields }
    }
}

#[async_trait]
impl Executor for RawCommandExecutor {
    fn name(&self) -> &'static str {
        "raw_command"
    }

    #[cfg(not(target_family = "wasm"))]
    #[instrument(level = "debug", name = "RawCommandExecutor::execute", skip(_state))]
    async fn execute(
        &self,
        _state: super::execute::ExecutorState,
        payload: FxHashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, ExecutorError> {
        let command = FIELD_COMMAND.extract_str(&payload)?;
        let mut cmd = tokio::process::Command::new(command.as_ref());

        // Don't leak our environment, which may contain secrets, to other commands.
        cmd.env_clear();
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let args = FIELD_ARGS.extract_string_array(&payload)?;
        for v in args {
            cmd.arg(v.as_ref());
        }

        let env = FIELD_ENV.extract_object(&payload)?;
        if let serde_json::Value::Object(m) = env.as_ref() {
            for (k, v) in m {
                let value = json_primitive_as_string("env", Some(k), &v, false)?;
                cmd.env(k, value.as_ref());
            }
        }

        let allow_failure: bool = FIELD_ALLOW_FAILURE.extract(&payload)?;

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
