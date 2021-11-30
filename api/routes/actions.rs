use actix_web::{
    delete, get, post, put,
    web::{self, Path},
    HttpResponse, Responder,
};
use ergo_auth::Authenticated;
use ergo_database::{
    object_id::{ActionCategoryId, ActionId},
    sql_insert_parameters,
};
use ergo_tasks::actions::{
    execute::{ScriptOrTemplate, EXECUTOR_REGISTRY},
    template::TemplateFields,
    Action,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::Connection;

use crate::{error::Result, web_app_server::AppStateData};

#[derive(Serialize, Clone, Debug, JsonSchema)]
pub struct ExecutorInfo<'a> {
    pub name: &'a str,
    pub template_fields: &'a TemplateFields,
}

#[get("/executors")]
pub async fn list_executors() -> Result<impl Responder> {
    let info = EXECUTOR_REGISTRY
        .iter()
        .map(|(_, exec)| ExecutorInfo {
            name: exec.name(),
            template_fields: exec.template_fields(),
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(info))
}

#[get("/actions")]
pub async fn list_actions(data: AppStateData) -> Result<impl Responder> {
    let actions = sqlx::query_as!(
        Action,
        r##"SELECT
        action_id as "action_id: ActionId",
        action_category_id as "action_category_id: ActionCategoryId",
        name,
        description,
        executor_id,
        executor_template as "executor_template: ScriptOrTemplate",
        template_fields as "template_fields: TemplateFields",
        timeout,
        postprocess_script,
        account_required,
        COALESCE(array_agg(account_type_id) FILTER(WHERE account_type_id IS NOT NULL), ARRAY[]::text[]) "account_types!"
        FROM actions
        LEFT JOIN allowed_action_account_types USING(action_id)
        GROUP BY action_id"##,
    )
    .fetch_all(&data.pg)
    .await?;

    Ok(HttpResponse::Ok().json(actions))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct ActionPayload {
    pub action_category_id: ActionCategoryId,
    pub name: String,
    pub description: Option<String>,
    pub executor_id: String,
    pub executor_template: ScriptOrTemplate,
    pub template_fields: TemplateFields,
    pub timeout: Option<i32>,
    /// A script that processes the executor's JSON result.
    /// The result is exposed in the variable `result` and the action's payload
    /// is exposed as `payload`. The value returned will replace the executor's
    /// return value, or an error can be thrown to mark the action as failed.
    pub postprocess_script: Option<String>,
    pub account_required: bool,
    #[serde(default)]
    pub account_types: Vec<String>,
}

impl ActionPayload {
    pub fn into_action(self, action_id: ActionId) -> Action {
        Action {
            action_id,
            action_category_id: self.action_category_id,
            name: self.name,
            description: self.description,
            executor_id: self.executor_id,
            executor_template: self.executor_template,
            template_fields: self.template_fields,
            timeout: self.timeout,
            postprocess_script: self.postprocess_script,
            account_required: self.account_required,
            account_types: self.account_types,
        }
    }
}

#[post("/actions")]
pub async fn new_action(
    data: AppStateData,
    auth: Authenticated,
    payload: web::Json<ActionPayload>,
) -> Result<impl Responder> {
    auth.expect_admin()?;

    let payload: Action = payload.into_inner().into_action(ActionId::new());
    payload
        .validate()
        .await
        .map_err(ergo_tasks::Error::ActionValidateError)?;

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    sqlx::query!(
        "INSERT INTO actions (action_id, action_category_id, name, description,
        executor_id, executor_template, template_fields, account_required,
        postprocess_script, timeout) VALUES
        ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        &payload.action_id.0,
        &payload.action_category_id.0,
        &payload.name,
        &payload.description as _,
        &payload.executor_id,
        sqlx::types::Json(&payload.executor_template) as _,
        sqlx::types::Json(&payload.template_fields) as _,
        &payload.account_required,
        payload.postprocess_script.as_ref(),
        payload.timeout,
    )
    .execute(&mut tx)
    .await?;

    if !payload.account_types.is_empty() {
        let q = format!(
            "INSERT INTO allowed_action_account_types (account_type_id, action_id) VALUES {}",
            sql_insert_parameters::<2>(payload.account_types.len())
        );

        let mut query = sqlx::query(&q);
        for account_type in &payload.account_types {
            query = query.bind(account_type).bind(&payload.action_id.0);
        }

        query.execute(&mut tx).await?;
    }

    tx.commit().await?;

    Ok(HttpResponse::Created().json(payload))
}

#[put("/actions/{action_id}")]
pub async fn write_action(
    data: AppStateData,
    auth: Authenticated,
    action_id: Path<ActionId>,
    payload: web::Json<ActionPayload>,
) -> Result<impl Responder> {
    auth.expect_admin()?;

    let payload: Action = payload.into_inner().into_action(action_id.into_inner());

    payload
        .validate()
        .await
        .map_err(ergo_tasks::Error::ActionValidateError)?;

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    sqlx::query!(
        "INSERT INTO actions (action_id, action_category_id, name, description,
            executor_id, executor_template, template_fields, account_required,
            postprocess_script, timeout)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT(action_id) DO UPDATE
        SET action_category_id=$2, name=$3, description=$4,
        executor_id=$5, executor_template=$6, template_fields=$7, account_required=$8,
        postprocess_script=$9",
        &payload.action_id.0,
        &payload.action_category_id.0,
        &payload.name,
        &payload.description as _,
        &payload.executor_id,
        sqlx::types::Json(&payload.executor_template) as _,
        sqlx::types::Json(&payload.template_fields) as _,
        &payload.account_required,
        payload.postprocess_script.as_ref(),
        payload.timeout
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!("DELETE FROM allowed_action_account_types WHERE action_id=$1 AND account_type_id <> ALL($2)",
        &payload.action_id.0,
        &payload.account_types).execute(&mut tx).await?;

    if !payload.account_types.is_empty() {
        let q = format!(
            "INSERT INTO allowed_action_account_types (account_type_id, action_id) VALUES {}
            ON CONFLICT DO NOTHING",
            sql_insert_parameters::<2>(payload.account_types.len())
        );

        let mut query = sqlx::query(&q);
        for account_type in &payload.account_types {
            query = query.bind(account_type).bind(&payload.action_id.0);
        }

        query.execute(&mut tx).await?;
    }

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(payload))
}

#[delete("/actions/{action_id}")]
pub async fn delete_action(
    data: AppStateData,
    auth: Authenticated,
    action_id: Path<ActionId>,
) -> Result<impl Responder> {
    auth.expect_admin()?;
    sqlx::query!(
        "DELETE FROM actions WHERE action_id=$1",
        action_id.into_inner().0
    )
    .execute(&data.pg)
    .await?;
    Ok(HttpResponse::Ok().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(list_actions)
        .service(new_action)
        .service(write_action)
        .service(delete_action)
        .service(list_executors);
}
