#[cfg(not(target_family = "wasm"))]
mod error;
#[cfg(not(target_family = "wasm"))]
mod pool;
#[cfg(not(target_family = "wasm"))]
pub mod redis;
#[cfg(not(target_family = "wasm"))]
pub mod transaction;

pub mod object_id;

#[cfg(not(target_family = "wasm"))]
pub use self::redis::RedisPool;
#[cfg(not(target_family = "wasm"))]
pub use error::*;
#[cfg(not(target_family = "wasm"))]
pub use pool::*;

pub fn new_uuid() -> uuid::Uuid {
    ulid::Ulid::new().into()
}

#[macro_export]
macro_rules! sqlx_json_decode {
    ($type:ty) => {
        impl<'r> sqlx::Decode<'r, sqlx::Postgres> for $type {
            fn decode(
                value: sqlx::postgres::PgValueRef<'r>,
            ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
                use sqlx::ValueRef;
                let is_jsonb = value.type_info().as_ref() == &sqlx::postgres::PgTypeInfo::with_oid(3802);
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
                sqlx::postgres::PgTypeInfo::with_oid(3802) // jsonb
            }

            fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
                *ty == sqlx::postgres::PgTypeInfo::with_oid(3802) // jsonb
                    || *ty == sqlx::postgres::PgTypeInfo::with_oid(114) // json
            }
        }
    };
}
