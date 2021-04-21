use chrono::{DateTime, Utc};
use fxhash::FxHashMap;
use sqlx::{types::Json, Postgres};

use crate::{database::PostgresPool, error::Error};

use super::{template::TemplateFields, ActionInvocation};

#[derive(Debug, sqlx::FromRow)]
struct ExecuteActionData {
    executor_id: String,
    executor_template_fields: Json<TemplateFields>,
    action_executor_template: Json<FxHashMap<String, serde_json::Value>>,
    action_template_fields: Json<TemplateFields>,
    account_required: bool,
    action_template: Option<Json<FxHashMap<String, serde_json::Value>>>,
    account_id: Option<i64>,
    account_fields: Option<Json<FxHashMap<String, serde_json::Value>>>,
    account_expires: Option<DateTime<Utc>>,
}

pub async fn execute(pg_pool: &PostgresPool, invocation: ActionInvocation) -> Result<(), Error> {
    let action: ExecuteActionData = sqlx::query_as(
        r##"SELECT
        executor_id,
        executors.template_fields as executor_template_fields,
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

    // 1. Merge the invocation payload with action_template and account_fields, if present.

    let mut action_payload = action
        .action_template
        .map(|t| t.0)
        .unwrap_or_else(FxHashMap::default);

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
    // 3. Apply the payload to executor_template.
    // 4. Send the executor payload to the executor to actually run it.

    Ok(())
}
