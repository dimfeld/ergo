use std::{borrow::Cow, collections::hash_map::RandomState};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fxhash::{FxBuildHasher, FxHashMap};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{types::Json, Postgres};
use thiserror::Error;
use tracing::{event, instrument, span, Instrument, Level};

use crate::{database::PostgresPool, tasks::actions::ActionStatus};

use super::{
    template::{self, TemplateFields},
    ActionInvocation,
};

pub fn json_primitive_as_string<'a>(
    field: &str,
    subfield: Option<&str>,
    value: &'a serde_json::Value,
    allow_missing: bool,
) -> Result<Cow<'a, String>, ExecutorError> {
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
) -> Result<Cow<'a, String>, ExecutorError> {
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

    /// Returns the template fields for the executor
    fn template_fields(&self) -> &TemplateFields;
}

lazy_static! {
    pub static ref EXECUTOR_REGISTRY: FxHashMap<String, Box<dyn Executor>> = {
        vec![
            super::http_executor::HttpExecutor::new(),
            super::raw_command_executor::RawCommandExecutor::new(),
        ]
        .into_iter()
        .map(|(name, ex)| (name.to_string(), ex))
        .collect::<FxHashMap<String, Box<dyn Executor>>>()
    };
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum ScriptOrTemplate {
    Template(Vec<(String, serde_json::Value)>),
    // Script(String),
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
    task_action_local_id: String,
    task_action_name: String,
    task_action_template: Option<Json<Vec<(String, serde_json::Value)>>>,
    account_id: Option<i64>,
    account_fields: Option<Json<Vec<(String, serde_json::Value)>>>,
    account_expires: Option<DateTime<Utc>>,
}

#[instrument(name = "execute_action", level = "debug")]
pub async fn execute(
    pg_pool: &PostgresPool,
    invocation: &ActionInvocation,
) -> Result<serde_json::Value, ExecuteError> {
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

    let result = execute_action(pg_pool, invocation).await;
    event!(Level::TRACE, ?result);

    let (status, response) = match &result {
        Ok(r) => (ActionStatus::Success, json!({ "output": r })),
        Err(e) => (
            ActionStatus::Error,
            json!({
                "error": e.to_string(),
                "info": format!("{:?}", e),
            }),
        ),
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

async fn execute_action(
    pg_pool: &PostgresPool,
    invocation: &ActionInvocation,
) -> Result<serde_json::Value, ExecuteError> {
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
        task_actions.task_action_local_id,
        task_actions.name as task_action_name,
        task_actions.action_template as task_action_template,
        task_actions.account_id,
        accounts.fields as account_fields,
        accounts.expires as account_expires

        FROM task_actions
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

    let executor = EXECUTOR_REGISTRY.get(&action.executor_id).ok_or_else(|| {
        ExecuteError::from_action_and_error(
            &action,
            ExecuteErrorSource::MissingExecutor(action.executor_id.clone()),
        )
    })?;

    let action_template_values = prepare_invocation(executor, &invocation.payload, &mut action)
        .map_err(|e| ExecuteError::from_action_and_error(&action, e))?;

    event!(Level::TRACE, ?action_template_values);

    // 4. Send the executor payload to the executor to actually run it.
    let results = executor
        .execute(pg_pool.clone(), action_template_values)
        .await
        .map_err(|e| ExecuteError::from_action_and_error(&action, e))?;

    Ok(results)
}

fn prepare_invocation(
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
    let action_template_values = match &action.action_executor_template.0 {
        ScriptOrTemplate::Template(t) => template::validate_and_apply(
            "action",
            action.action_id,
            &action.action_template_fields.0,
            &t,
            &action_payload,
        )?,
    };

    // 3. Make sure the resulting template matches what the executor expects.
    template::validate(
        "executor",
        &action.executor_id,
        executor.template_fields(),
        &action_template_values,
    )?;

    // 5. Write results to the action log. Retry on failure.

    Ok(action_template_values)
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
        async fn execute(
            &self,
            pg_pool: PostgresPool,
            template_values: FxHashMap<String, Value>,
        ) -> Result<Value, ExecutorError> {
            todo!()
        }

        fn template_fields(&self) -> &TemplateFields {
            &self.template_fields
        }
    }

    #[test]
    fn test_simple() {}
}
