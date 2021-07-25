use crate::common::run_database_test;
use ergo_api::database::object_id;

#[actix_rt::test]
async fn new_object_id() {
    run_database_test(|db| async move {
        let mut conn = db.pool.acquire().await?;
        let id1 = object_id::new_object_id(&mut conn, "task").await?;
        let id2 = object_id::new_object_id(&mut conn, "task").await?;

        assert_ne!(id1, id2, "sequentially created IDs are different");

        Ok(())
    })
    .await;
}

#[actix_rt::test]
async fn new_object_id_with_value_disallows_duplicates_when_asked() {
    run_database_test(|db| async move {
        let mut conn = db.pool.acquire().await?;
        let id = object_id::new_object_id_with_value(&mut conn, Some(10000), "task", false).await?;
        assert_eq!(id, 10000, "ID is created with requested value");

        let id2 =
            object_id::new_object_id_with_value(&mut conn, Some(10001), "task", false).await?;
        assert_eq!(id2, 10001, "ID is created with requested value");

        let result =
            object_id::new_object_id_with_value(&mut conn, Some(10000), "task", false).await;
        result
            .err()
            .expect("Creating duplicate ID fails when allow_existing is false");

        Ok(())
    })
    .await;
}

#[actix_rt::test]
async fn new_object_id_with_value_allows_duplicates_when_asked() {
    run_database_test(|db| async move {
        let mut conn = db.pool.acquire().await?;
        let id = object_id::new_object_id_with_value(&mut conn, Some(10000), "task", false).await?;
        assert_eq!(id, 10000, "ID is created with requested value");

        let id2 =
            object_id::new_object_id_with_value(&mut conn, Some(10001), "task", false).await?;
        assert_eq!(id2, 10001, "ID is created with requested value");

        let id3 = object_id::new_object_id_with_value(&mut conn, Some(10000), "task", true).await?;
        assert_eq!(id3, 10000, "Duplicate requested ID gets same value");

        let result =
            object_id::new_object_id_with_value(&mut conn, Some(10001), "action", true).await;
        result.err().expect(
            "Creating duplicate ID fails when allow_existing is true but the type doesn't match",
        );

        Ok(())
    })
    .await;
}
