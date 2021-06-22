use crate::common::run_app_test;

#[actix_rt::test]
async fn smoke_test() {
    run_app_test(|app| async move {
        let response = app.admin_user.client.get("tasks").send().await?;

        assert_eq!(
            response.status().as_u16(),
            200,
            "response status code should be 200"
        );
        Ok(())
    })
    .await;
}
