mod error;
#[cfg(not(target_family = "wasm"))]
mod pool;
#[cfg(not(target_family = "wasm"))]
pub mod redis;
#[cfg(not(target_family = "wasm"))]
pub mod transaction;

pub mod object_id;

use std::env;

#[cfg(not(target_family = "wasm"))]
pub use self::redis::RedisPool;
pub use error::*;
#[cfg(not(target_family = "wasm"))]
pub use pool::*;

#[cfg(not(target_family = "wasm"))]
pub mod test;

#[derive(Clone, Debug)]
pub struct DatabaseConfiguration {
    pub host: String,
    pub port: u16,
    pub database: String,
}

impl Default for DatabaseConfiguration {
    fn default() -> Self {
        database_configuration_from_env().unwrap()
    }
}

pub fn database_configuration_from_env() -> Result<DatabaseConfiguration, Error> {
    Ok(DatabaseConfiguration {
        host: env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: envoption::with_default("DATABASE_PORT", 5432_u16)
            .map_err(|e| Error::ConfigError(e.to_string()))?,
        database: env::var("DATABASE").unwrap_or_else(|_| "ergo".to_string()),
    })
}

pub fn new_uuid() -> uuid::Uuid {
    ulid::Ulid::new().into()
}

#[cfg(not(target_family = "wasm"))]
pub const JSON_OID: sqlx::postgres::types::Oid = sqlx::postgres::types::Oid(114);
#[cfg(not(target_family = "wasm"))]
pub const JSONB_OID: sqlx::postgres::types::Oid = sqlx::postgres::types::Oid(3802);

#[macro_export]
macro_rules! sqlx_json_decode {
    ($type:ty) => {
        impl<'r> sqlx::Decode<'r, sqlx::Postgres> for $type {
            fn decode(
                value: sqlx::postgres::PgValueRef<'r>,
            ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
                use sqlx::ValueRef;
                let is_jsonb = value.type_info().as_ref()
                    == &sqlx::postgres::PgTypeInfo::with_oid($crate::JSONB_OID);
                let mut buf = <&[u8] as sqlx::Decode<sqlx::Postgres>>::decode(value)?;

                if is_jsonb {
                    assert_eq!(
                        buf[0], 1,
                        "unsupported JSONB format version {}; please open an issue",
                        buf[0]
                    );

                    buf = &buf[1..];
                }
                serde_json::from_slice(buf).map_err(Into::into)
            }
        }

        impl sqlx::Type<sqlx::Postgres> for $type {
            fn type_info() -> sqlx::postgres::PgTypeInfo {
                sqlx::postgres::PgTypeInfo::with_oid($crate::JSONB_OID)
            }

            fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
                *ty == sqlx::postgres::PgTypeInfo::with_oid($crate::JSONB_OID)
                    || *ty == sqlx::postgres::PgTypeInfo::with_oid($crate::JSON_OID)
            }
        }
    };
}
