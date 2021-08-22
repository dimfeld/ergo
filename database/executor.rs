use async_stream::try_stream;
use either::Either;
use futures::{
    future::BoxFuture,
    stream::{BoxStream, TryStreamExt},
};
use sqlx::{
    database::HasStatement,
    postgres::{PgQueryResult, PgRow},
    Database, Describe, Error, Execute, Executor, PgConnection, Postgres,
};

use super::RenewablePostgresPool;

// This is modified from the Pool Executor implementation in sqlx.
// Currently disabled until I figure out some lifetime problems.

impl<'p> Executor<'p> for &'_ RenewablePostgresPool
where
    for<'c> &'c mut PgConnection: Executor<'c, Database = Postgres>,
{
    type Database = Postgres;

    fn fetch_many<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxStream<'e, Result<Either<PgQueryResult, PgRow>, Error>>
    where
        E: Execute<'q, Self::Database>,
    {
        let pool = self.clone();

        Box::pin(try_stream! {
            let mut conn = pool.acquire().await?;
            let mut s = conn.conn.fetch_many(query);

            while let Some(v) = s.try_next().await? {
                yield v;
            }
        })
    }

    fn fetch_optional<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<Option<PgRow>, Error>>
    where
        E: Execute<'q, Self::Database>,
    {
        let pool = self.clone();

        Box::pin(async move { pool.acquire().await?.fetch_optional(query).await })
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Self::Database as Database>::TypeInfo],
    ) -> BoxFuture<'e, Result<<Self::Database as HasStatement<'q>>::Statement, Error>> {
        let pool = self.clone();

        Box::pin(async move { pool.acquire().await?.prepare_with(sql, parameters).await })
    }

    #[doc(hidden)]
    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> BoxFuture<'e, Result<Describe<Self::Database>, Error>> {
        let pool = self.clone();

        Box::pin(async move { pool.acquire().await?.describe(sql).await })
    }
}
