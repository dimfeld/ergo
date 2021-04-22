use std::collections::hash_map::RandomState;

use chrono::{DateTime, Utc};
use fxhash::{FxBuildHasher, FxHashMap};
use sqlx::{types::Json, Postgres};

use crate::{database::PostgresPool, error::Error};

use super::{
    template::{self, TemplateFields},
    ActionInvocation,
};

#[derive(Debug, sqlx::FromRow)]
struct ExecuteActionData {
    executor_id: String,
    executor_template_fields: Json<TemplateFields>,
    action_id: i64,
    action_name: String,
    action_executor_template: Json<Vec<(String, serde_json::Value)>>,
    action_template_fields: Json<TemplateFields>,
    account_required: bool,
    action_template: Option<Json<Vec<(String, serde_json::Value)>>>,
    account_id: Option<i64>,
    account_fields: Option<Json<Vec<(String, serde_json::Value)>>>,
    account_expires: Option<DateTime<Utc>>,
}

pub async fn execute(pg_pool: &PostgresPool, invocation: ActionInvocation) -> Result<(), Error> {
    let action: ExecuteActionData = sqlx::query_as(
        r##"SELECT
        executor_id,
        executors.template_fields as executor_template_fields,
        action_id,
        actions.name as action_name,
        actions.executor_template as action_executor_template,
        actions.template_fields as action_template_fields,
        actions.account_required,
        task_actions.action_template as action_template,
        task_actions.account_id,
        accounts.fields as account_fields,
        accounts.expires as account_expires

        FROM task_actions
        JOIN actions USING(action_id)
        JOIN executors USING(executor_id)
        LEFT JOIN accounts USING(account_id)

        WHERE task_action_id=$1"##,
    )
    .bind(invocation.task_action_id)
    .fetch_one(pg_pool)
    .await?;

    if action.account_required && action.account_id.is_none() {
        // TODO Real Error
        return Err(Error::StringError(format!(
            "Action {} requires an account",
            &action.action_name
        )));
    }

    // 1. Merge the invocation payload with action_template and account_fields, if present.

    let mut action_payload = FxHashMap::with_capacity_and_hasher(
        action.action_template_fields.0.len(),
        FxBuildHasher::default(),
    );

    if let serde_json::Value::Object(invocation_payload) = invocation.payload {
        for (k, v) in invocation_payload {
            action_payload.insert(k, v);
        }
    }

    if let Some(account_fields) = action.account_fields {
        for (k, v) in account_fields.0 {
            action_payload.insert(k, v);
        }
    }

    // 2. Verify that it all matches the action template_fields.
    let action_template_values = template::validate_and_apply(
        "action",
        action.action_id,
        &action.action_template_fields.0,
        &action.action_executor_template,
        &action_payload,
    )?;

    // 3. Apply the filled-in action template to the executor template.
    let executor_payload = template::validate(
        "executor",
        action.executor_id,
        &action.executor_template_fields.0,
        &action_template_values,
    )?;

    // 4. Send the executor payload to the executor to actually run it.

    Ok(())
}
