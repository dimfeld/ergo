use futures::{future::BoxFuture, stream::BoxStream};
use sqlx::{
    database::HasStatement,
    postgres::{PgQueryResult, PgRow},
    Database, Describe, Error, Execute, Executor, Postgres,
};

impl<'c> Executor<'c> for &'c mut crate::WrappedConnection {
    type Database = Postgres;

    #[inline]
    fn fetch_many<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxStream<'e, Result<either::Either<PgQueryResult, PgRow>, Error>>
    where
        'c: 'e,
        E: Execute<'q, Postgres>,
    {
        (**self).fetch_many(query)
    }

    #[inline]
    fn fetch_optional<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<Option<PgRow>, Error>>
    where
        'c: 'e,
        E: Execute<'q, Postgres>,
    {
        (**self).fetch_optional(query)
    }

    #[inline]
    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Postgres as Database>::TypeInfo],
    ) -> BoxFuture<'e, Result<<Postgres as HasStatement<'q>>::Statement, Error>>
    where
        'c: 'e,
    {
        (**self).prepare_with(sql, parameters)
    }

    #[doc(hidden)]
    #[inline]
    fn describe<'e, 'q: 'e>(self, sql: &'q str) -> BoxFuture<'e, Result<Describe<Postgres>, Error>>
    where
        'c: 'e,
    {
        (**self).describe(sql)
    }
}
