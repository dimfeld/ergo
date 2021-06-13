use crate::common;
use ergo::database::transaction;
use futures::future::FutureExt;
use sqlx::Row;

#[actix_rt::test]
async fn handles_serialization_error() {
    common::run_database_test(|db| async move {
        sqlx::query("CREATE TABLE txtest (id bigint primary key, value bigint not null)")
            .execute(&db.pool)
            .await?;
        sqlx::query("INSERT INTO txtest (id, value) VALUES (1, 0)")
            .execute(&db.pool)
            .await?;

        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
        let mut tasks = Vec::new();
        for _ in 0..3 {
            let barrier = barrier.clone();
            let pool = db.pool.clone();
            tasks.push(tokio::task::spawn(async move {
                let mut conn = pool.acquire().await?;
                barrier.wait().await;
                transaction::serializable(&mut conn, 3, |tx| {
                    async move {
                        sqlx::query("UPDATE txtest SET value = value + 1 WHERE id = 1")
                            .execute(tx)
                            .await
                    }
                    .boxed()
                })
                .await
            }));
        }

        let results = futures::future::join_all(tasks).await;
        for result in results {
            result.unwrap().unwrap();
        }

        let row = sqlx::query("SELECT value FROM txtest WHERE id=1")
            .fetch_one(&db.pool)
            .await?;
        let value: i64 = row.get(0);
        assert_eq!(value, 3, "each task incremented the value");

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn bails_on_error() {
    common::run_database_test(|db| async move {
        let mut conn = db.pool.acquire().await?;
        let result = transaction::serializable(&mut conn, 1, |_| {
            async move { Err::<(), _>(ergo::error::Error::StringError("An error".to_string())) }
                .boxed()
        })
        .await;

        result.unwrap_err();
        Ok(())
    })
    .await;
}
