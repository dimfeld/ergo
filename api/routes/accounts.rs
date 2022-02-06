use actix_web::{get, web, HttpResponse, Responder};
use ergo_auth::Authenticated;
use ergo_database::object_id::AccountId;
use ergo_tasks::actions::accounts::{AccountPublicInfo, AccountType};

use crate::{error::Result, web_app_server::AppStateData};

#[get("/account_types")]
pub async fn list_account_types(data: AppStateData) -> Result<impl Responder> {
    let account_types = sqlx::query_as!(
        AccountType,
        r##"SELECT account_type_id, name, description, COALESCE(fields, ARRAY[]::text[]) as "fields!" FROM account_types"##
    )
    .fetch_all(&data.pg)
    .await?;

    Ok(HttpResponse::Ok().json(account_types))
}

#[get("/accounts")]
pub async fn list_accounts(data: AppStateData, auth: Authenticated) -> Result<impl Responder> {
    let accounts = sqlx::query_as!(
        AccountPublicInfo,
        r##"SELECT account_id AS "account_id: AccountId", account_type_id, name FROM accounts
            WHERE org_id=$1"##,
        auth.org_id().0
    )
    .fetch_all(&data.pg)
    .await?;

    Ok(HttpResponse::Ok().json(accounts))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(list_account_types).service(list_accounts);
}
