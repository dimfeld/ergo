use actix_web::{
    delete, get, post, put,
    web::{self, Data, Path},
    HttpRequest, HttpResponse, Responder,
};
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::Connection;

use crate::{
    auth::Authenticated,
    database::{object_id::new_object_id_with_value, sql_insert_parameters},
    error::{Error, Result},
    tasks::inputs::Input,
    web_app_server::AppStateData,
};

use super::{
    execute::ScriptOrTemplate,
    template::{validate, TemplateFields},
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ActionPayload {
    action_id: Option<i64>,
    action_category_id: i64,
    name: String,
    description: Option<String>,
    executor_id: String,
    executor_template: ScriptOrTemplate,
    template_fields: TemplateFields,
    account_required: bool,
    account_types: Option<Vec<String>>,
}

impl ActionPayload {
    fn validate(&self) -> Result<()> {
        match (
            super::execute::EXECUTOR_REGISTRY.get(&self.executor_id),
            &self.executor_template,
        ) {
            (Some(executor), ScriptOrTemplate::Template(values)) => {
                let values_map = values.iter().cloned().collect::<FxHashMap<_, _>>();
                validate(
                    "action",
                    &self.action_id.unwrap_or(-1),
                    executor.template_fields(),
                    &values_map,
                )?;

                Ok(())
            }
            (None, _) => Err(Error::UnknownExecutor(self.executor_id.clone())),
        }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ActionDescription {
    action_id: i64,
    action_category_id: i64,
    name: String,
    description: Option<String>,
    template_fields: sqlx::types::Json<TemplateFields>,
    account_required: bool,
    account_types: Option<Vec<String>>,
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
    payload.validate()?;

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    let action_id = new_object_id_with_value(&mut tx, payload.action_id.as_ref()).await?;
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

    payload.validate()?;

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    sqlx::query!(
        "UPDATE actions SET action_category_id=$2, name=$3, description=$4,
        executor_id=$5, executor_template=$6, template_fields=$7, account_required=$8
        WHERE action_id=$1",
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
    sqlx::query!(
        "DELETE FROM actions WHERE action_id=$1",
        action_id.into_inner()
    )
    .execute(&data.pg)
    .await?;
    Ok(HttpResponse::Ok().finish())
}
