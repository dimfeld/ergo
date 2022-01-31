#[cfg(not(target_family = "wasm"))]
use crate::inputs::{enqueue_input, EnqueueInputOptions};

use super::{
    execute::Executor,
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
#[cfg(not(target_family = "wasm"))]
use ergo_auth::get_user_info;
use ergo_database::object_id::{InputId, TaskId, TaskTriggerId};
#[cfg(not(target_family = "wasm"))]
use sqlx::Connection;

static FIELD_TASK: TemplateField = TemplateField::from_static(
    "task",
    TemplateFieldFormat::string_without_default(),
    false,
    "The task to send the input to",
);

static FIELD_TRIGGER: TemplateField = TemplateField::from_static(
    "trigger_name",
    TemplateFieldFormat::string_without_default(),
    false,
    "The local ID of the task's trigger",
);

static FIELD_TIME: TemplateField = TemplateField::from_static(
    "time",
    TemplateFieldFormat::string_without_default(),
    true,
    "When to send the input",
);

static FIELD_PAYLOAD: TemplateField = TemplateField::from_static(
    "payload",
    TemplateFieldFormat::object_without_default(true),
    true,
    "The payload to send to the trigger",
);

#[derive(Debug)]
pub struct SendInputExecutor {
    template_fields: TemplateFields,
}

impl SendInputExecutor {
    pub fn new() -> SendInputExecutor {
        let template_fields = [&FIELD_TRIGGER, &FIELD_TASK, &FIELD_TIME, &FIELD_PAYLOAD].into();
        SendInputExecutor { template_fields }
    }
}

#[async_trait]
impl Executor for SendInputExecutor {
    #[cfg(not(target_family = "wasm"))]
    async fn execute(
        &self,
        state: super::execute::ExecutorState,
        template_values: fxhash::FxHashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, super::execute::ExecutorError> {
        use super::execute::ExecutorError;

        let task_id = TaskId::from_str(FIELD_TASK.extract_str(&template_values)?.as_ref())
            .map_err(|e| ExecutorError::FieldFormatError {
                field: FIELD_TASK.name.to_string(),
                subfield: None,
                expected: e.to_string(),
            })?;
        let trigger_name = FIELD_TRIGGER.extract_str(&template_values)?;

        let time_arg = FIELD_TIME.extract_str(&template_values)?;
        let when = if time_arg.len() > 0 {
            let parsed_time = DateTime::parse_from_rfc3339(time_arg.as_ref())
                .map(|t| t.with_timezone(&Utc))
                .map_err(|e| ExecutorError::FieldFormatError {
                    field: FIELD_TIME.name.to_string(),
                    subfield: None,
                    expected: e.to_string(),
                })?;
            Some(parsed_time)
        } else {
            None
        };

        let payload = FIELD_PAYLOAD.extract_object(&template_values)?.into_owned();

        let pg_pool = state.pg_pool.ok_or(ExecutorError::MissingDatabase)?;
        let mut conn = pg_pool
            .acquire()
            .await
            .map_err(ExecutorError::command_error_without_result)?;
        let mut tx = conn
            .begin()
            .await
            .map_err(ExecutorError::command_error_without_result)?;
        let user = get_user_info(&mut tx, &state.user_id, None)
            .await
            .map_err(ExecutorError::command_error_without_result)?;

        let data = sqlx::query!(
            r##"SELECT tasks.task_id as "task_id: TaskId",
                tasks.name as task_name,
                tt.name as task_trigger_name,
                task_trigger_id as "task_trigger_id: TaskTriggerId",
                input_id as "input_id: InputId",
                inputs.payload_schema
            FROM task_triggers tt
            JOIN tasks USING(task_id)
            JOIN inputs USING (input_id)
            WHERE org_id=$2 AND task_trigger_local_id = $3 AND task_id=$4 AND EXISTS (
                SELECT 1 FROM user_entity_permissions
                WHERE user_entity_id = ANY($1)
                AND permission_type = 'trigger_event'
                AND permissioned_object IN (uuid_nil(), task_trigger_id)
            )"##,
            user.user_entity_ids.as_slice(),
            &user.org_id.0,
            trigger_name.as_ref(),
            &task_id.0,
        )
        .fetch_one(&mut tx)
        .await
        .map_err(ExecutorError::command_error_without_result)?;

        let mut conn = pg_pool
            .acquire()
            .await
            .map_err(ExecutorError::command_error_without_result)?;
        enqueue_input(EnqueueInputOptions {
            pg: &mut conn,
            notifications: None,
            org_id: user.org_id.clone(),
            user_id: user.user_id.clone(),
            task_id,
            input_id: data.input_id,
            task_trigger_id: data.task_trigger_id,
            task_trigger_local_id: trigger_name.to_string(),
            task_trigger_name: data.task_trigger_name,
            task_name: data.task_name,
            payload_schema: &data.payload_schema,
            payload,
            redis_key_prefix: state.redis_key_prefix.as_deref(),
            trigger_at: when,
            periodic_trigger_id: None,
        })
        .await
        .map_err(ExecutorError::command_error_without_result)?;

        Ok(serde_json::Value::Null)
    }

    fn name(&self) -> &'static str {
        "send_input"
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}
