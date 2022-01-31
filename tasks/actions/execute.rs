use std::borrow::Cow;

use async_trait::async_trait;
#[cfg(not(target_family = "wasm"))]
use ergo_database::PostgresPool;
use ergo_database::{object_id::UserId, sqlx_json_decode};
use fxhash::FxHashMap;
use lazy_static::lazy_static;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    template::{TemplateFields, TemplateValidationFailure},
    TaskActionTemplate,
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

    #[error("Executor requires database connection but none was provided")]
    MissingDatabase,

    #[error("Error during command execution: {source}")]
    CommandError {
        source: anyhow::Error,
        result: serde_json::Value,
    },
}

impl ExecutorError {
    pub fn command_error_without_result(err: impl Into<anyhow::Error>) -> ExecutorError {
        ExecutorError::CommandError {
            source: err.into(),
            result: serde_json::Value::Null,
        }
    }
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

#[derive(Clone, Debug)]
#[cfg(not(target_family = "wasm"))]
pub struct ExecutorState {
    pub pg_pool: Option<PostgresPool>,
    pub redis_key_prefix: Option<String>,
    pub user_id: UserId,
}

#[cfg(test)]
impl ExecutorState {
    pub fn new_test_state() -> Self {
        ExecutorState {
            pg_pool: None,
            redis_key_prefix: None,
            user_id: UserId::new(),
        }
    }
}

#[async_trait]
pub trait Executor: std::fmt::Debug + Send + Sync {
    #[cfg(not(target_family = "wasm"))]
    async fn execute(
        &self,
        state: ExecutorState,
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
            Box::new(super::send_input_executor::SendInputExecutor::new()) as Box<dyn Executor>,
        ])
        .map(|e| (e.name(), e))
        .collect::<FxHashMap<&'static str, Box<dyn Executor>>>()
    };
}

#[derive(Clone, Debug, JsonSchema, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum ScriptOrTemplate {
    Template(TaskActionTemplate),
    Script(String),
}

#[cfg(not(target_family = "wasm"))]
sqlx_json_decode!(ScriptOrTemplate);

#[cfg(not(target_family = "wasm"))]
pub use native::*;

#[cfg(not(target_family = "wasm"))]
mod native {
    use chrono::{DateTime, Utc};
    use ergo_database::{
        object_id::{AccountId, ActionId, OrgId, TaskId},
        PostgresPool,
    };
    use ergo_notifications::{Notification, NotificationManager, NotifyEvent};
    use futures::future::TryFutureExt;
    use fxhash::{FxBuildHasher, FxHashMap};
    use serde_json::json;
    use sqlx::types::Json;
    use thiserror::Error;
    use tracing::{event, instrument, Level};

    use crate::{
        actions::{
            template::{self, TemplateError, TemplateFields},
            ActionInvocation, ActionStatus,
        },
        error::Error,
        scripting::{self, run_simple_with_args},
    };

    use super::*;

    #[instrument(name = "execute_action", level = "debug", skip(pg_pool, notifications))]
    pub async fn execute(
        pg_pool: &PostgresPool,
        redis_key_prefix: Option<String>,
        notifications: Option<&NotificationManager>,
        invocation: ActionInvocation,
    ) -> Result<serde_json::Value, Error> {
        event!(Level::DEBUG, ?invocation);

        sqlx::query!(
            "UPDATE actions_log SET status='running', updated=now() WHERE actions_log_id=$1",
            &invocation.actions_log_id
        )
        .execute(pg_pool)
        .await
        .map_err(|e| ExecuteError {
            task_id: invocation.task_id.clone(),
            task_action_local_id: invocation.task_action_local_id.clone(),
            // We don't actually know the task action name here, but that's ok.
            task_action_name: String::new(),
            error: e.into(),
        })?;

        let result = execute_action(pg_pool, redis_key_prefix, notifications, &invocation).await;
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
            &invocation.actions_log_id,
            status as _,
            response
        )
        .execute(pg_pool)
        .await
        .map_err(|e| ExecuteError {
            task_id: invocation.task_id.clone(),
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
        action_id: ActionId,
        action_name: String,
        action_executor_template: Json<ScriptOrTemplate>,
        action_template_fields: Json<TemplateFields>,
        account_required: bool,
        postprocess_script: Option<String>,
        task_id: TaskId,
        task_name: String,
        task_action_local_id: String,
        task_action_name: String,
        task_action_template: Option<Json<TaskActionTemplate>>,
        account_id: Option<AccountId>,
        account_fields: Option<Json<TaskActionTemplate>>,
        account_expires: Option<DateTime<Utc>>,
        org_id: OrgId,
        run_as: Option<UserId>,
    }

    async fn execute_action(
        pg_pool: &PostgresPool,
        redis_key_prefix: Option<String>,
        notifications: Option<&NotificationManager>,
        invocation: &ActionInvocation,
    ) -> Result<serde_json::Value, Error> {
        let task_id = &invocation.task_id;
        let task_action_local_id = &invocation.task_action_local_id;
        let mut action: ExecuteActionData = sqlx::query_as_unchecked!(
            ExecuteActionData,
            r##"SELECT
        executor_id,
        action_id as "action_id: ActionId",
        actions.name as action_name,
        actions.executor_template as action_executor_template,
        actions.template_fields as action_template_fields,
        actions.account_required,
        actions.postprocess_script,
        task_id as "task_id: TaskId",
        tasks.name AS task_name,
        task_actions.task_action_local_id,
        task_actions.name as task_action_name,
        NULLIF(task_actions.action_template, 'null'::jsonb) as task_action_template,
        task_actions.account_id as "account_id: AccountId",
        NULLIF(accounts.fields, 'null'::jsonb) as account_fields,
        accounts.expires as account_expires,
        tasks.org_id as "org_id: OrgId",
        tasks.run_as as "run_as: Option<UserId>"

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
            task_id: task_id.clone(),
            task_action_local_id: invocation.task_action_local_id.clone(),
            task_action_name: String::new(),
            error: e.into(),
        })?;

        event!(Level::DEBUG, ?action);
        event!(Level::INFO,
            %task_id,
            %action.task_action_local_id,
            %action.action_id,
            %action.action_name,
            %action.task_action_name,
            "executing action"
        );

        if let Some(notifications) = &notifications {
            let mut conn = pg_pool.acquire().await?;
            let notification = Notification {
                task_id: action.task_id.clone(),
                event: NotifyEvent::ActionStarted,
                payload: Some(invocation.payload.clone()),
                error: None,
                task_name: action.task_name.clone(),
                log_id: Some(invocation.actions_log_id),
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

        let prepare_action = PrepareInvocationAction {
            executor_id: action.executor_id.as_str(),
            action_id: &action.action_id,
            account_id: &action.account_id,
            account_fields: action.account_fields.clone().map(|t| t.0),
            account_required: action.account_required,
            account_expires: action.account_expires,
            task_action_template: action.task_action_template.clone().map(|t| t.0),
            action_template_fields: &action.action_template_fields,
            action_executor_template: &action.action_executor_template,
        };

        let prepare_result =
            validate_and_prepare_invocation(executor, &invocation.payload, prepare_action)
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

        event!(Level::DEBUG, ?action_template_values);

        // Send the executor payload to the executor to actually run it.
        let postprocess = action.postprocess_script.as_ref();
        let executor_state = ExecutorState {
            pg_pool: Some(pg_pool.clone()),
            redis_key_prefix,
            user_id: action
                .run_as
                .take()
                .unwrap_or_else(|| invocation.user_id.clone()),
        };

        let results = executor
            .execute(executor_state, action_template_values)
            .map_err(|e| e.into())
            .and_then(|result| async move {
                match postprocess {
                    Some(script) => {
                        let processed: serde_json::Value = run_simple_with_args(
                            script,
                            &[("output", &result), ("payload", &invocation.payload)],
                        )
                        .await
                        .map_err(ExecuteErrorSource::ScriptError)?;

                        if !processed.is_null() {
                            Ok(processed)
                        } else {
                            // If the postprocess script didn't return anything then just
                            // return the original result.
                            Ok::<serde_json::Value, ExecuteErrorSource>(result)
                        }
                    }
                    None => Ok(result),
                }
            })
            .await
            .map_err(|e| ExecuteError::from_action_and_error(&action, e));

        match results {
            Ok(results) => {
                if let Some(notifications) = notifications {
                    event!(Level::INFO,
                        %task_id,
                        %action.task_action_local_id,
                        %action.action_id,
                        %action.action_name,
                        %action.task_action_name,
                        "action succceeded"
                    );
                    let notification = Notification {
                        task_id: invocation.task_id.clone(),
                        event: NotifyEvent::ActionSuccess,
                        payload: Some(invocation.payload.clone()),
                        error: None,
                        task_name: action.task_name.clone(),
                        log_id: Some(invocation.actions_log_id),
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
                event!(Level::ERROR,
                    %task_id,
                    %action.task_action_local_id,
                    %action.action_id,
                    %action.action_name,
                    %action.task_action_name,
                    err=?e,
                    "action failed"
                );
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
    ) -> Result<(), Error> {
        if let Some(notifications) = notifications {
            let notification = Notification {
                task_id: invocation.task_id.clone(),
                event: NotifyEvent::ActionError,
                payload: Some(invocation.payload.clone()),
                error: Some(error.to_string()),
                task_name: action.task_name.clone(),
                log_id: Some(invocation.actions_log_id),
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

    #[derive(Debug)]
    pub struct PrepareInvocationAction<'a> {
        pub action_id: &'a ActionId,
        pub action_template_fields: &'a TemplateFields,
        pub action_executor_template: &'a ScriptOrTemplate,
        pub task_action_template: Option<TaskActionTemplate>,
        pub executor_id: &'a str,
        pub account_required: bool,
        pub account_id: &'a Option<AccountId>,
        pub account_fields: Option<TaskActionTemplate>,
        pub account_expires: Option<DateTime<Utc>>,
    }

    pub async fn validate_and_prepare_invocation(
        executor: &Box<dyn Executor>,
        invocation_payload: &serde_json::Value,
        mut action: PrepareInvocationAction<'_>,
    ) -> Result<FxHashMap<String, serde_json::Value>, ExecuteErrorSource> {
        match (
            action.account_required,
            action.account_id,
            action.account_expires,
        ) {
            (true, None, _) => return Err(ExecuteErrorSource::AccountRequired),
            (_, Some(account_id), Some(expires)) => {
                if expires < Utc::now() {
                    return Err(ExecuteErrorSource::AccountExpired(account_id.clone()));
                }
            }
            _ => {}
        };

        // 1. Merge the invocation payload with action_template and account_fields, if present.

        let mut action_payload = FxHashMap::with_capacity_and_hasher(
            action.action_template_fields.0.len()
                + action
                    .task_action_template
                    .as_ref()
                    .map(|t| t.len())
                    .unwrap_or(0)
                + action.account_fields.as_ref().map(|f| f.len()).unwrap_or(0),
            FxBuildHasher::default(),
        );

        // Create the payload in this ordeR:
        // 1. Task action template
        // 2. Action invocation payload
        // 3. Account fields
        //
        // This allows the invocation payload to overwrite the values in the task action template, but the
        // account fields must take precedence over everything else.

        if let Some(task_action_fields) = action.task_action_template.take() {
            for (k, v) in task_action_fields {
                action_payload.insert(k, v);
            }
        }

        if let serde_json::Value::Object(invocation_payload) = invocation_payload {
            for (k, v) in invocation_payload {
                action_payload.insert(k.clone(), v.clone());
            }
        }

        if let Some(account_fields) = action.account_fields.take() {
            for (k, v) in account_fields {
                action_payload.insert(k, v);
            }
        }

        event!(Level::DEBUG, ?action, ?action_payload);

        // 2. Verify that it all matches the action template_fields.
        let action_template_values = match action.action_executor_template {
            ScriptOrTemplate::Template(t) => template::validate_and_apply(
                "action",
                &action.action_id,
                action.action_template_fields,
                t,
                &action_payload,
            )?,
            ScriptOrTemplate::Script(s) => {
                let s = s.clone();
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
            Some(&action.executor_id),
            executor.template_fields(),
            &action_template_values,
        )?;

        Ok(action_template_values)
    }

    #[derive(Debug, Error)]
    #[error("Action {task_action_name} ({task_id}:{task_action_local_id}): {error}")]
    pub struct ExecuteError {
        pub task_id: TaskId,
        pub task_action_local_id: String,
        pub task_action_name: String,
        pub error: ExecuteErrorSource,
    }

    impl ExecuteError {
        fn from_action_and_error(
            action: &ExecuteActionData,
            error: impl Into<ExecuteErrorSource>,
        ) -> Self {
            ExecuteError {
                task_id: action.task_id.clone(),
                task_action_local_id: action.task_action_local_id.clone(),
                task_action_name: action.task_action_name.clone(),
                error: error.into(),
            }
        }
    }

    #[derive(Debug, Error)]
    pub enum ExecuteErrorSource {
        #[error(transparent)]
        TemplateError(#[from] TemplateError),

        #[error("Script error: {0}")]
        ScriptError(anyhow::Error),

        #[error(transparent)]
        ExecutorError(#[from] ExecutorError),

        #[error("Unknown executor {0}")]
        MissingExecutor(String),

        #[error("Action requires an account")]
        AccountRequired,

        #[error("Account {0} is expired")]
        AccountExpired(AccountId),

        #[error("SQL Error")]
        SqlError(#[from] sqlx::error::Error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::{json, Value};

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
            _state: ExecutorState,
            _template_values: FxHashMap<String, Value>,
        ) -> Result<Value, ExecutorError> {
            Ok(self.return_value.clone())
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
}
