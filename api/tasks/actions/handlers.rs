use actix_web::{
    delete, get, post, put,
    web::{self, Path},
    HttpResponse, Responder,
};
use futures::future::ready;
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::Connection;

use crate::{
    auth::Authenticated,
    database::{object_id::new_object_id_with_value, sql_insert_parameters},
    error::{Error, Result},
    tasks::scripting,
    web_app_server::AppStateData,
};

use super::{
    execute::ScriptOrTemplate,
    template::{validate, TemplateFields},
};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ActionPayload {
    pub action_id: Option<i64>,
    pub action_category_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub executor_id: String,
    pub executor_template: ScriptOrTemplate,
    pub template_fields: TemplateFields,
    pub account_required: bool,
    pub account_types: Option<Vec<String>>,
}

impl ActionPayload {
    async fn validate(&self) -> Result<()> {
        let executor = super::execute::EXECUTOR_REGISTRY
            .get(self.executor_id.as_str())
            .ok_or_else(|| Error::UnknownExecutor(self.executor_id.clone()))?;

        let values_map = match &self.executor_template {
            ScriptOrTemplate::Template(values) => {
                values.iter().cloned().collect::<FxHashMap<_, _>>()
            }
            ScriptOrTemplate::Script(s) => {
                let s = s.clone();
                scripting::POOL
                    .run(move || {
                        let mut runtime = scripting::create_simple_runtime();
                        let values = runtime
                            .run_expression::<FxHashMap<String, serde_json::Value>>(
                                "<executor template>",
                                &s,
                            );
                        ready(values)
                    })
                    .await
                    .map_err(Error::ScriptError)?
            }
        };

        validate(
            "action",
            &self.action_id.unwrap_or(-1),
            executor.template_fields(),
            &values_map,
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, sqlx::FromRow, PartialEq, Eq)]
pub struct ActionDescription {
    pub action_id: i64,
    pub action_category_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub template_fields: sqlx::types::Json<TemplateFields>,
    pub account_required: bool,
    pub account_types: Option<Vec<String>>,
}

#[get("/actions")]
pub async fn list_actions(data: AppStateData) -> Result<impl Responder> {
    let actions = sqlx::query_as_unchecked!(
        ActionDescription,
        "SELECT action_id, action_category_id, name, description,
        template_fields,
        account_required,
        array_agg(account_type_id) FILTER(WHERE account_type_id IS NOT NULL) account_types
        FROM actions
        LEFT JOIN allowed_action_account_types USING(action_id)
        GROUP BY action_id",
    )
    .fetch_all(&data.pg)
    .await?;

    Ok(HttpResponse::Ok().json(actions))
}

#[post("/actions")]
pub async fn new_action(
    data: AppStateData,
    auth: Authenticated,
    payload: web::Json<ActionPayload>,
) -> Result<impl Responder> {
    auth.expect_admin()?;

    let payload = payload.into_inner();
    payload.validate().await?;

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    let action_id = new_object_id_with_value(&mut tx, payload.action_id, "action", false).await?;
    sqlx::query!(
        "INSERT INTO actions (action_id, action_category_id, name, description,
        executor_id, executor_template, template_fields, account_required) VALUES
        ($1, $2, $3, $4, $5, $6, $7, $8)",
        action_id,
        &payload.action_category_id,
        &payload.name,
        &payload.description as _,
        &payload.executor_id,
        sqlx::types::Json(&payload.executor_template) as _,
        sqlx::types::Json(&payload.template_fields) as _,
        &payload.account_required
    )
    .execute(&mut tx)
    .await?;

    if let Some(account_types) = payload.account_types.as_ref() {
        if !account_types.is_empty() {
            let q = format!(
                "INSERT INTO allowed_action_account_types (account_type_id, action_id) VALUES {}",
                sql_insert_parameters::<2>(account_types.len())
            );

            let mut query = sqlx::query(&q);
            for account_type in account_types {
                query = query.bind(account_type).bind(action_id);
            }

            query.execute(&mut tx).await?;
        }
    }

    tx.commit().await?;

    let output = ActionDescription {
        action_id,
        action_category_id: payload.action_category_id,
        name: payload.name,
        description: payload.description,
        template_fields: sqlx::types::Json(payload.template_fields),
        account_types: payload.account_types,
        account_required: payload.account_required,
    };

    Ok(HttpResponse::Created().json(output))
}

#[put("/actions/{action_id}")]
pub async fn write_action(
    data: AppStateData,
    auth: Authenticated,
    action_id: Path<i64>,
    payload: web::Json<ActionPayload>,
) -> Result<impl Responder> {
    auth.expect_admin()?;

    let action_id = action_id.into_inner();
    let payload = payload.into_inner();

    payload.validate().await?;

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    new_object_id_with_value(&mut tx, Some(action_id), "action", true).await?;

    sqlx::query!(
        "INSERT INTO actions (action_id, action_category_id, name, description,
            executor_id, executor_template, template_fields, account_required)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT(action_id) DO UPDATE
        SET action_category_id=$2, name=$3, description=$4,
        executor_id=$5, executor_template=$6, template_fields=$7, account_required=$8",
        action_id,
        &payload.action_category_id,
        &payload.name,
        &payload.description as _,
        &payload.executor_id,
        sqlx::types::Json(&payload.executor_template) as _,
        sqlx::types::Json(&payload.template_fields) as _,
        &payload.account_required
    )
    .execute(&mut tx)
    .await?;

    let account_types = payload.account_types.unwrap_or_else(Vec::new);

    if !account_types.is_empty() {
        let q = format!(
            "INSERT INTO allowed_action_account_types (account_type_id, action_id) VALUES {}
            ON CONFLICT DO NOTHING",
            sql_insert_parameters::<2>(account_types.len())
        );

        let mut query = sqlx::query(&q);
        for account_type in &account_types {
            query = query.bind(account_type).bind(action_id);
        }

        query.execute(&mut tx).await?;
    }

    sqlx::query!("DELETE FROM allowed_action_account_types WHERE action_id=$1 AND account_type_id <> ALL($2)",
        action_id,
        &account_types).execute(&mut tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().finish())
}

#[delete("/actions/{action_id}")]
pub async fn delete_action(
    data: AppStateData,
    auth: Authenticated,
    action_id: Path<i64>,
) -> Result<impl Responder> {
    auth.expect_admin()?;
    sqlx::query!(
        "DELETE FROM actions WHERE action_id=$1",
        action_id.into_inner()
    )
    .execute(&data.pg)
    .await?;
    Ok(HttpResponse::Ok().finish())
}
