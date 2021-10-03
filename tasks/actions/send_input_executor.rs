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

const FIELD_TASK: TemplateField = TemplateField::from_static(
    "task",
    TemplateFieldFormat::String,
    false,
    "The task to send the input to",
);

const FIELD_TRIGGER: TemplateField = TemplateField::from_static(
    "trigger_name",
    TemplateFieldFormat::String,
    false,
    "The local ID of the task's trigger",
);

const FIELD_TIME: TemplateField = TemplateField::from_static(
    "time",
    TemplateFieldFormat::String,
    true,
    "When to send the input",
);

const FIELD_PAYLOAD: TemplateField = TemplateField::from_static(
    "payload",
    TemplateFieldFormat::Object,
    true,
    "The payload to send to the trigger",
);

#[derive(Debug)]
pub struct SendInputExecutor {
    template_fields: TemplateFields,
}

impl SendInputExecutor {
    pub fn new() -> SendInputExecutor {
        let template_fields = vec![FIELD_TRIGGER, FIELD_TASK, FIELD_TIME, FIELD_PAYLOAD].into();
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

        let task_id = TaskId::from_str(FIELD_TASK.extract_str(&template_values)?.unwrap())
            .map_err(|e| ExecutorError::FieldFormatError {
                field: FIELD_TASK.name.to_string(),
                subfield: None,
                expected: e.to_string(),
            })?;
        let trigger_name = FIELD_TRIGGER.extract_str(&template_values)?.unwrap_or("");
        let when = FIELD_TIME
            .extract_str(&template_values)?
            .map(|s| DateTime::parse_from_rfc3339(s))
            .transpose()
            .map_err(|e| ExecutorError::FieldFormatError {
                field: FIELD_TIME.name.to_string(),
                subfield: None,
                expected: e.to_string(),
            })?
            .map(|t| t.with_timezone(&Utc));
        let payload = FIELD_PAYLOAD
            .extract_object(&template_values)?
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        let mut conn = state
            .pg_pool
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
            &trigger_name,
            &task_id.0,
        )
        .fetch_one(&mut tx)
        .await
        .map_err(ExecutorError::command_error_without_result)?;

        enqueue_input(EnqueueInputOptions {
            pg: &state.pg_pool,
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
            redis_key_prefix: &state.redis_key_prefix,
            trigger_at: when,
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
