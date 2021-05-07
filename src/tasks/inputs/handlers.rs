use actix_web::{
    delete, get, post, put,
    web::{self, Data, Path},
    HttpRequest, HttpResponse, Responder,
};
use serde::Deserialize;
use sqlx::Connection;

use crate::{
    auth::Authenticated,
    database::object_id::new_object_id_with_value,
    error::{Error, Result},
    tasks::inputs::Input,
    web_app_server::AppStateData,
};

#[derive(Debug, Deserialize)]
pub struct InputPayload {
    input_id: Option<i64>,
    input_category_id: Option<i64>,
    name: String,
    description: Option<String>,
    payload_schema: serde_json::Value,
}

#[get("/inputs")]
pub async fn list_inputs(
    data: AppStateData,
    // _auth: Authenticated,
) -> Result<impl Responder> {
    let inputs = sqlx::query_as!(Input, "SELECT * FROM inputs")
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

    let payload = payload.into_inner();

    // Make sure the schema is valid.
    jsonschema::JSONSchema::compile(&payload.payload_schema)?;

    let mut conn = data.pg.acquire().await?;
    let mut tx = conn.begin().await?;

    let input_id = new_object_id_with_value(&mut tx, payload.input_id.as_ref()).await?;
    sqlx::query!(
        "INSERT INTO inputs (input_id, input_category_id, name, description, payload_schema) VALUES
        ($1, $2, $3, $4, $5)",
        input_id,
        &payload.input_category_id as _,
        &payload.name,
        &payload.description as _,
        &payload.payload_schema
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Created().json(Input {
        input_id,
        input_category_id: payload.input_category_id,
        name: payload.name,
        description: payload.description,
        payload_schema: payload.payload_schema,
    }))
}

#[put("/inputs/{input_id}")]
pub async fn write_input(
    data: AppStateData,
    input_id: Path<i64>,
    payload: web::Json<InputPayload>,
    auth: Authenticated,
) -> Result<impl Responder> {
    auth.expect_admin()?;

    let payload = payload.into_inner();
    let input_id = input_id.into_inner();

    // Make sure the schema is valid.
    jsonschema::JSONSchema::compile(&payload.payload_schema)?;

    sqlx::query!(
        "INSERT INTO inputs (input_id, input_category_id, name, description, payload_schema) VALUES
        ($1, $2, $3, $4, $5)
        ON CONFLICT(input_id) DO UPDATE SET input_category_id=$2, name=$3, description=$4, payload_schema=$5",
        input_id,
        &payload.input_category_id as _,
        &payload.name,
        &payload.description as _,
        &payload.payload_schema
    )
    .execute(&data.pg)
    .await?;

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
    input_id: Path<i64>,
    auth: Authenticated,
) -> Result<impl Responder> {
    auth.expect_admin()?;
    let input_id = input_id.into_inner();

    sqlx::query!("DELETE FROM inputs WHERE input_id=$1", input_id)
        .execute(&data.pg)
        .await?;
    Ok(HttpResponse::Ok().finish())
}
