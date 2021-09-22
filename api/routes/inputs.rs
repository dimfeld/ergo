use actix_web::{
    delete, get, post, put,
    web::{self, Path},
    HttpResponse, Responder,
};
use ergo_auth::Authenticated;
use ergo_database::object_id::{InputCategoryId, InputId};
use ergo_tasks::inputs::Input;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::Connection;

use crate::{error::Result, web_app_server::AppStateData};

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
pub struct InputPayload {
    pub input_category_id: Option<InputCategoryId>,
    pub name: String,
    pub description: Option<String>,
    pub payload_schema: serde_json::Value,
}

impl InputPayload {
    pub fn into_input(self, input_id: InputId) -> Input {
        Input {
            input_id,
            input_category_id: self.input_category_id,
            name: self.name,
            description: self.description,
            payload_schema: self.payload_schema,
        }
    }
}

#[get("/inputs")]
pub async fn list_inputs(data: AppStateData) -> Result<impl Responder> {
    let inputs = sqlx::query_as!(
        Input,
        r##"SELECT
            input_id as "input_id: InputId",
            input_category_id as "input_category_id: InputCategoryId",
            name, description, payload_schema
        FROM inputs"##
    )
    .fetch_all(&data.pg)
    .await?;
    Ok(HttpResponse::Ok().json(inputs))
}

#[post("/inputs")]
pub async fn new_input(
    data: AppStateData,
    payload: web::Json<InputPayload>,
    auth: Authenticated,
) -> Result<impl Responder> {
    auth.expect_admin()?;

    let payload = payload.into_inner().into_input(InputId::new());

    // Make sure the schema is valid.
    jsonschema::JSONSchema::compile(&payload.payload_schema)?;

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    sqlx::query!(
        "INSERT INTO inputs (input_id, input_category_id, name, description, payload_schema) VALUES
        ($1, $2, $3, $4, $5)",
        &payload.input_id.0,
        &payload.input_category_id as _,
        &payload.name,
        &payload.description as _,
        &payload.payload_schema
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Created().json(payload))
}

#[put("/inputs/{input_id}")]
pub async fn write_input(
    data: AppStateData,
    input_id: Path<InputId>,
    payload: web::Json<InputPayload>,
    auth: Authenticated,
) -> Result<impl Responder> {
    auth.expect_admin()?;

    let payload = payload.into_inner();
    let input_id = input_id.into_inner();

    // Make sure the schema is valid.
    jsonschema::JSONSchema::compile(&payload.payload_schema)?;

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    sqlx::query!(
        "INSERT INTO inputs (input_id, input_category_id, name, description, payload_schema)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT(input_id) DO UPDATE
        SET input_category_id=$2, name=$3, description=$4, payload_schema=$5",
        &input_id.0,
        &payload.input_category_id as _,
        &payload.name,
        &payload.description as _,
        &payload.payload_schema
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(Input {
        input_id,
        input_category_id: payload.input_category_id,
        name: payload.name,
        description: payload.description,
        payload_schema: payload.payload_schema,
    }))
}

#[delete("/inputs/{input_id}")]
pub async fn delete_input(
    data: AppStateData,
    input_id: Path<InputId>,
    auth: Authenticated,
) -> Result<impl Responder> {
    auth.expect_admin()?;
    let input_id = input_id.into_inner();

    sqlx::query!("DELETE FROM inputs WHERE input_id=$1", input_id.0)
        .execute(&data.pg)
        .await?;
    Ok(HttpResponse::Ok().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(list_inputs)
        .service(new_input)
        .service(write_input)
        .service(delete_input);
}
