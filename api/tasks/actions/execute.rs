use std::borrow::Cow;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ergo_database::PostgresPool;
use fxhash::{FxBuildHasher, FxHashMap};
use lazy_static::lazy_static;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::types::Json;
use thiserror::Error;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    notifications::{Notification, NotificationManager, NotifyEvent},
    tasks::{actions::ActionStatus, scripting},
};

use super::{
    template::{self, TemplateFields, TemplateValidationFailure},
    ActionInvocation,
};

pub fn json_primitive_as_string<'a>(
    field: &str,
    subfield: Option<&str>,
    value: &'a serde_json::Value,
    allow_missing: bool,
) -> Result<Cow<'a, str>, ExecutorError> {
    match value {
        serde_json::Value::String(s) => Ok(Cow::Borrowed(s)),
        serde_json::Value::Number(n) => Ok(Cow::Owned(n.to_string())),
        serde_json::Value::Bool(b) => Ok(Cow::Owned(b.to_string())),
        serde_json::Value::Null => {
            if allow_missing {
                Ok(Cow::Owned(String::new()))
            } else {
                Err(ExecutorError::MissingFieldError(field.to_string()))
            }
        }
        _ => Err(ExecutorError::FieldFormatError {
            field: field.to_string(),
            subfield: subfield.map(|sf| sf.to_string()),
            expected: "primitive value".to_string(),
        }),
    }
}

pub fn get_primitive_payload_value<'a>(
    values: &'a FxHashMap<String, serde_json::Value>,
    name: &str,
    allow_missing: bool,
) -> Result<Cow<'a, str>, ExecutorError> {
    match values.get(name) {
        Some(n) => json_primitive_as_string(name, None, n, allow_missing),
        None => {
            if allow_missing {
                Ok(Cow::Owned(String::new()))
            } else {
                Err(ExecutorError::MissingFieldError(name.to_string()))
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("Missing field {0}")]
    MissingFieldError(String),

    #[error("Expected field {field}{} to be a {expected}",
        if let Some(s) = .subfield { format!("[{}]", s) } else { String::new() }
        )]
    FieldFormatError {
        field: String,
        subfield: Option<String>,
        expected: String,
    },

    #[error("Error during command execution: {source}")]
    CommandError {
        source: anyhow::Error,
        result: serde_json::Value,
    },
}

impl From<TemplateValidationFailure> for ExecutorError {
    fn from(e: TemplateValidationFailure) -> Self {
        match e {
            TemplateValidationFailure::Required(s) => Self::MissingFieldError(s.to_string()),
            TemplateValidationFailure::Invalid { name, expected, .. } => Self::FieldFormatError {
                field: name.to_string(),
                expected: expected.to_string(),
                subfield: None,
            },
        }
    }
}

#[derive(Debug, Error)]
#[error("Action {task_action_name} ({task_id}:{task_action_local_id}): {error}")]
pub struct ExecuteError {
    task_id: i64,
    task_action_local_id: String,
    task_action_name: String,
    error: ExecuteErrorSource,
}

impl ExecuteError {
    fn from_action_and_error(
        action: &ExecuteActionData,
        error: impl Into<ExecuteErrorSource>,
    ) -> Self {
        ExecuteError {
            task_id: action.task_id,
            task_action_local_id: action.task_action_local_id.clone(),
            task_action_name: action.task_action_name.clone(),
            error: error.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ExecuteErrorSource {
    #[error(transparent)]
    TemplateError(#[from] super::template::TemplateError),

    #[error("Script error: {0}")]
    ScriptError(anyhow::Error),

    #[error(transparent)]
    ExecutorError(#[from] ExecutorError),

    #[error("Unknown executor {0}")]
    MissingExecutor(String),

    #[error("Action requires an account")]
    AccountRequired,

    #[error("SQL Error")]
    SqlError(#[from] sqlx::error::Error),
}

#[async_trait]
pub trait Executor: std::fmt::Debug + Send + Sync {
    async fn execute(
        &self,
        pg_pool: PostgresPool,
        template_values: FxHashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, ExecutorError>;

    fn name(&self) -> &'static str;

    /// Returns the template fields for the executor
    fn template_fields(&self) -> &TemplateFields;
}

lazy_static! {
    pub static ref EXECUTOR_REGISTRY: FxHashMap<&'static str, Box<dyn Executor>> = {
        std::array::IntoIter::new([
            Box::new(super::http_executor::HttpExecutor::new()) as Box<dyn Executor>,
            Box::new(super::raw_command_executor::RawCommandExecutor::new()) as Box<dyn Executor>,
            Box::new(super::js_executor::JsExecutor::new()) as Box<dyn Executor>,
        ])
        .map(|e| (e.name(), e))
        .collect::<FxHashMap<&'static str, Box<dyn Executor>>>()
    };
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum ScriptOrTemplate {
    Template(Vec<(String, serde_json::Value)>),
    Script(String),
}

#[instrument(name = "execute_action", level = "debug", skip(pg_pool, notifications))]
pub async fn execute(
    pg_pool: &PostgresPool,
    notifications: Option<&crate::notifications::NotificationManager>,
    invocation: &ActionInvocation,
) -> Result<serde_json::Value> {
    event!(Level::DEBUG, ?invocation);

    let actions_log_id = invocation.actions_log_id.clone();
    sqlx::query!(
        "UPDATE actions_log SET status='running', updated=now() WHERE actions_log_id=$1",
        invocation.actions_log_id
    )
    .execute(pg_pool)
    .await
    .map_err(|e| ExecuteError {
        task_id: invocation.task_id,
        task_action_local_id: invocation.task_action_local_id.clone(),
        // We don't actually know the task action name here, but that's ok.
        task_action_name: String::new(),
        error: e.into(),
    })?;

    let result = execute_action(pg_pool, notifications, invocation).await;
    event!(Level::DEBUG, ?result);

    let (status, response) = match &result {
        Ok(r) => (ActionStatus::Success, json!({ "output": r })),
        Err(e) => {
            event!(Level::ERROR, err=?e, "Action error");
            (
                ActionStatus::Error,
                json!({
                    "error": e.to_string(),
                    "info": format!("{:?}", e),
                }),
            )
        }
    };

    sqlx::query!(
        "UPDATE actions_log SET status=$2, result=$3, updated=now()
        WHERE actions_log_id=$1",
        actions_log_id,
        status as _,
        response
    )
    .execute(pg_pool)
    .await
    .map_err(|e| ExecuteError {
        task_id: invocation.task_id,
        task_action_local_id: invocation.task_action_local_id.clone(),
        // We don't actually know the task action name here, but that's ok.
        task_action_name: String::new(),
        error: e.into(),
    })?;

    result
}

#[derive(Debug, sqlx::FromRow)]
struct ExecuteActionData {
    executor_id: String,
    action_id: i64,
    action_name: String,
    action_executor_template: Json<ScriptOrTemplate>,
    action_template_fields: Json<TemplateFields>,
    account_required: bool,
    task_id: i64,
    task_name: String,
    task_action_local_id: String,
    task_action_name: String,
    task_action_template: Option<Json<Vec<(String, serde_json::Value)>>>,
    account_id: Option<i64>,
    account_fields: Option<Json<Vec<(String, serde_json::Value)>>>,
    account_expires: Option<DateTime<Utc>>,
    org_id: Uuid,
}

async fn execute_action(
    pg_pool: &PostgresPool,
    notifications: Option<&crate::notifications::NotificationManager>,
    invocation: &ActionInvocation,
) -> Result<serde_json::Value, Error> {
    let task_id = invocation.task_id;
    let task_action_local_id = &invocation.task_action_local_id;
    let mut action: ExecuteActionData = sqlx::query_as_unchecked!(
        ExecuteActionData,
        r##"SELECT
        executor_id,
        action_id,
        actions.name as action_name,
        actions.executor_template as action_executor_template,
        actions.template_fields as action_template_fields,
        actions.account_required,
        task_id,
        tasks.name AS task_name,
        task_actions.task_action_local_id,
        task_actions.name as task_action_name,
        NULLIF(task_actions.action_template, 'null'::jsonb) as task_action_template,
        task_actions.account_id,
        NULLIF(accounts.fields, 'null'::jsonb) as account_fields,
        accounts.expires as account_expires,
        tasks.org_id

        FROM task_actions
        JOIN tasks USING (task_id)
        JOIN actions USING(action_id)
        LEFT JOIN accounts USING(account_id)

        WHERE task_id=$1 AND task_action_local_id=$2"##,
        task_id,
        &task_action_local_id
    )
    .fetch_one(pg_pool)
    .await
    .map_err(|e| ExecuteError {
        task_id,
        task_action_local_id: invocation.task_action_local_id.clone(),
        task_action_name: String::new(),
        error: e.into(),
    })?;

    event!(Level::TRACE, ?action);
    event!(Level::INFO,
        task_id,
        %action.task_action_local_id,
        %action.action_id,
        %action.action_name,
        %action.task_action_name,
        "executing action"
    );

    if let Some(notifications) = &notifications {
        let mut conn = pg_pool.acquire().await?;
        let notification = Notification {
            task_id: action.task_id,
            event: NotifyEvent::ActionStarted,
            payload: Some(invocation.payload.clone()),
            error: None,
            task_name: action.task_name.clone(),
            log_id: Some(invocation.actions_log_id.clone()),
            local_id: action.task_action_local_id.clone(),
            local_object_id: None,
            local_object_name: action.task_action_name.clone(),
        };

        notifications
            .notify(&mut conn, &action.org_id, notification)
            .await?;
    }

    let executor = EXECUTOR_REGISTRY
        .get(action.executor_id.as_str())
        .ok_or_else(|| {
            ExecuteError::from_action_and_error(
                &action,
                ExecuteErrorSource::MissingExecutor(action.executor_id.clone()),
            )
        })?;

    let prepare_result = prepare_invocation(executor, &invocation.payload, &mut action)
        .await
        .map(|values| (executor, values))
        .map_err(|e| ExecuteError::from_action_and_error(&action, e));

    let (executor, action_template_values) = match prepare_result {
        Ok(v) => v,
        Err(e) => {
            notify_action_error(pg_pool, notifications, invocation, action, &e).await?;
            return Err(e.into());
        }
    };

    event!(Level::TRACE, ?action_template_values);

    // Send the executor payload to the executor to actually run it.
    let results = executor
        .execute(pg_pool.clone(), action_template_values)
        .await
        .map_err(|e| ExecuteError::from_action_and_error(&action, e));

    match results {
        Ok(results) => {
            if let Some(notifications) = notifications {
                let notification = Notification {
                    task_id: invocation.task_id,
                    event: NotifyEvent::ActionError,
                    payload: Some(invocation.payload.clone()),
                    error: None,
                    task_name: action.task_name.clone(),
                    log_id: Some(invocation.actions_log_id.clone()),
                    local_id: action.task_action_local_id.clone(),
                    local_object_id: None,
                    local_object_name: action.task_action_name.clone(),
                };
                let mut conn = pg_pool.acquire().await?;
                notifications
                    .notify(&mut conn, &action.org_id, notification)
                    .await?;
            }

            Ok(results)
        }
        Err(e) => {
            notify_action_error(pg_pool, notifications, invocation, action, &e).await?;
            Err(e.into())
        }
    }
}

async fn notify_action_error(
    pool: &PostgresPool,
    notifications: Option<&NotificationManager>,
    invocation: &ActionInvocation,
    action: ExecuteActionData,
    error: &ExecuteError,
) -> Result<()> {
    if let Some(notifications) = notifications {
        let notification = Notification {
            task_id: invocation.task_id,
            event: NotifyEvent::ActionError,
            payload: Some(invocation.payload.clone()),
            error: Some(error.to_string()),
            task_name: action.task_name.clone(),
            log_id: Some(invocation.actions_log_id.clone()),
            local_id: action.task_action_local_id.clone(),
            local_object_id: None,
            local_object_name: action.task_action_name.clone(),
        };

        let mut conn = pool.acquire().await?;
        notifications
            .notify(&mut conn, &action.org_id, notification)
            .await?;
    }

    Ok(())
}

async fn prepare_invocation(
    executor: &Box<dyn Executor>,
    invocation_payload: &serde_json::Value,
    action: &mut ExecuteActionData,
) -> Result<FxHashMap<String, serde_json::Value>, ExecuteErrorSource> {
    if action.account_required && action.account_id.is_none() {
        return Err(ExecuteErrorSource::AccountRequired);
    }

    // 1. Merge the invocation payload with action_template and account_fields, if present.

    let mut action_payload = FxHashMap::with_capacity_and_hasher(
        action.action_template_fields.0.len(),
        FxBuildHasher::default(),
    );

    if let Some(task_action_fields) = action.task_action_template.take() {
        for (k, v) in task_action_fields.0 {
            action_payload.insert(k, v);
        }
    }

    if let serde_json::Value::Object(invocation_payload) = invocation_payload {
        for (k, v) in invocation_payload {
            action_payload.insert(k.clone(), v.clone());
        }
    }

    if let Some(account_fields) = action.account_fields.take() {
        for (k, v) in account_fields.0 {
            action_payload.insert(k, v);
        }
    }

    // 2. Verify that it all matches the action template_fields.
    let action_template_values = match action.action_executor_template.0.clone() {
        ScriptOrTemplate::Template(t) => template::validate_and_apply(
            "action",
            action.action_id,
            &action.action_template_fields.0,
            &t,
            &action_payload,
        )?,
        ScriptOrTemplate::Script(s) => {
            scripting::POOL
                .run(move || async move {
                    let mut runtime = scripting::create_simple_runtime();
                    runtime
                        .set_global_value("args", &action_payload)
                        .map_err(ExecuteErrorSource::ScriptError)?;
                    let values: FxHashMap<String, serde_json::Value> = runtime
                        .run_expression("<action executor template>", s.as_str())
                        .map_err(ExecuteErrorSource::ScriptError)?;
                    Ok::<_, ExecuteErrorSource>(values)
                })
                .await?
        }
    };

    // 3. Make sure the resulting template matches what the executor expects.
    template::validate(
        "executor",
        &action.executor_id,
        executor.template_fields(),
        &action_template_values,
    )?;

    Ok(action_template_values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::Value;

    #[derive(Debug)]
    struct MockExecutor {
        template_fields: TemplateFields,
        return_value: serde_json::Value,
    }

    #[async_trait]
    impl Executor for MockExecutor {
        fn name(&self) -> &'static str {
            "mock"
        }

        async fn execute(
            &self,
            _pg_pool: PostgresPool,
            template_values: FxHashMap<String, Value>,
        ) -> Result<Value, ExecutorError> {
            todo!()
        }

        fn template_fields(&self) -> &TemplateFields {
            &self.template_fields
        }
    }

    #[test]
    fn json_primitive_as_string() {
        assert_eq!(
            super::json_primitive_as_string("a", None, &json!("abc"), false).expect("string"),
            "abc",
            "string"
        );
        assert_eq!(
            super::json_primitive_as_string("a", None, &json!(3), false).expect("number"),
            "3",
            "number"
        );
        assert_eq!(
            super::json_primitive_as_string("a", None, &json!(true), false).expect("boolean"),
            "true",
            "boolean"
        );
        assert_eq!(
            super::json_primitive_as_string("a", None, &json!(null), true)
                .expect("allow_missing=true: null converts to empty string"),
            "",
            "allow_missing=true: null convers to empty string"
        );

        super::json_primitive_as_string("a", None, &json!(null), false)
            .expect_err("allow_missing=false: null throws error");
        super::json_primitive_as_string("a", None, &json!({"abc": 5}), false)
            .expect_err("object throws error");
        super::json_primitive_as_string("a", None, &json!([1, 2, 3]), false)
            .expect_err("array throws error");
    }

    #[test]
    fn get_primitive_payload_value() {
        let values = std::array::IntoIter::new([
            ("string_field", json!("a string")),
            ("number_field", json!(3)),
            ("bool_field", json!(true)),
            ("null_field", json!(null)),
            ("object_field", json!({ "d": 4 })),
            ("array_field", json!([1, 2, 3])),
        ])
        .map(|(k, v)| (k.to_string(), v))
        .collect::<FxHashMap<String, serde_json::Value>>();

        assert_eq!(
            super::get_primitive_payload_value(&values, "string_field", false)
                .expect("string field"),
            "a string",
            "string field"
        );
        assert_eq!(
            super::get_primitive_payload_value(&values, "number_field", false)
                .expect("number field"),
            "3",
            "number field"
        );
        assert_eq!(
            super::get_primitive_payload_value(&values, "bool_field", false).expect("bool field"),
            "true",
            "bool field"
        );

        super::get_primitive_payload_value(&values, "object_field", false)
            .expect_err("object field throws error");
        super::get_primitive_payload_value(&values, "array_field", false)
            .expect_err("array field throws error");

        super::get_primitive_payload_value(&values, "null_field", false)
            .expect_err("allow_missing=false: null value should return error");
        super::get_primitive_payload_value(&values, "missing", false)
            .expect_err("allow_missing=false: missing field should return error");

        assert_eq!(
            super::get_primitive_payload_value(&values, "null_field", true)
                .expect("allow_missing=true: null value converts to empty string"),
            "",
            "allow_missing=true: null value converts to empty string"
        );
        assert_eq!(
            super::get_primitive_payload_value(&values, "missing", true)
                .expect("allow_missing=true: missing field converts to empty string"),
            "",
            "allow_missing=true: null value converts to empty string"
        );
    }

    #[test]
    #[ignore]
    fn passing_action() {}

    #[test]
    #[ignore]
    fn failing_action() {}
}
