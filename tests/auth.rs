mod common;

use common::run_app_test;

#[actix_rt::test]
async fn anonymous_unauthenticated_endpoint() {
    run_app_test(|app| async move {
        let response = app.client.get("actions").send().await?.error_for_status()?;
        Ok(())
    })
    .await;
}

#[test]
#[ignore]
fn auth_by_api_key() {}

#[test]
#[ignore]
fn auth_by_cookie() {}

#[test]
#[ignore]
fn unknown_session_id() {}

#[test]
#[ignore]
fn deleted_user() {}

#[test]
#[ignore]
fn deleted_org() {}
