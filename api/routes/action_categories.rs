use actix_web::{get, web, HttpResponse, Responder};
use ergo_database::object_id::ActionCategoryId;
use ergo_tasks::actions::ActionCategory;

use crate::{error::Result, web_app_server::AppStateData};

#[get("/action_categories")]
pub async fn list_action_categories(data: AppStateData) -> Result<impl Responder> {
    let categories = sqlx::query_as!(
        ActionCategory,
        r##"SELECT
        action_category_id as "action_category_id: ActionCategoryId",
        name, description
        FROM action_categories"##
    )
    .fetch_all(&data.pg)
    .await?;

    Ok(HttpResponse::Ok().json(categories))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(list_action_categories);
}
