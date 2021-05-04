use crate::{
    database::PostgresPool,
    error::{Error, Result},
};
use actix_web::{dev::ServiceRequest, http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sqlx::{postgres::PgRow, query, query::Query, Encode, FromRow, Postgres};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKey {
    pub api_key_id: Uuid,
    pub prefix: String,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>,
    pub inherits_user_permissions: bool,
    pub description: Option<String>,
    pub active: bool,
    pub expires: Option<DateTime<Utc>>,
    pub created: DateTime<Utc>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ApiKeyAuth {
    pub api_key_id: Uuid,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>,
    pub inherits_user_permissions: bool,
}

pub struct KeyAndHash {
    api_key_id: Uuid,
    key: String,
    hash: Vec<u8>,
}

impl KeyAndHash {
    pub fn new(salt: &str) -> KeyAndHash {
        let id = Uuid::new_v4();
        let base64_id = base64::encode_config(id.as_bytes(), base64::URL_SAFE_NO_PAD);
        let random = base64::encode_config(Uuid::new_v4().as_bytes(), base64::URL_SAFE_NO_PAD);
        let key = format!("er1.{}.{}", base64_id, random);

        let mut hasher = sha2::Sha512::default();
        hasher.update(key.as_bytes());
        hasher.update(salt.as_bytes());
        let hash = hasher.finalize().to_vec();

        KeyAndHash {
            api_key_id: id,
            key,
            hash,
        }
    }

    pub fn from_key(salt: &str, token: &str) -> Result<(Uuid, Vec<u8>)> {
        if !token.starts_with("er1.") || token.len() != 49 {
            return Err(Error::AuthenticationError);
        }

        let mut hasher = sha2::Sha512::default();
        hasher.update(token.as_bytes());
        hasher.update(salt.as_bytes());
        let hash = hasher.finalize().to_vec();

        let id_portion = token
            .split('.')
            .skip(1)
            .next()
            .ok_or(Error::AuthenticationError)?;
        let api_key_bytes = base64::decode_config(id_portion.as_bytes(), base64::URL_SAFE_NO_PAD)
            .map_err(|_| Error::AuthenticationError)?;
        let api_key_id = Uuid::from_slice(&api_key_bytes)?;

        Ok((api_key_id, hash))
    }
}

#[derive(Deserialize)]
struct ApiQueryString {
    api_key: String,
}

async fn handle_api_key(
    pg: &PostgresPool,
    salt: &str,
    key: &str,
) -> Result<super::AuthenticationInfo> {
    let (api_key_id, hash) = KeyAndHash::from_key(salt, key)?;
    let auth_key = sqlx::query_as!(
        ApiKeyAuth,
        "SELECT api_key_id, org_id, user_id, inherits_user_permissions
        FROM api_keys
        WHERE api_key_id=$1 AND hash=$2 AND active AND (expires IS NULL OR expires < now())
        LIMIT 1",
        api_key_id,
        hash
    )
    .fetch_one(pg)
    .await?;

    let user = match &auth_key.user_id {
        None => None,
        // This could be combined with the query above, but for simplicity we just keep it separate
        // for now.
        Some(id) => Some(super::get_user_info(pg, id).await?),
    };

    Ok(super::AuthenticationInfo::ApiKey {
        key: auth_key,
        user,
    })
}

pub async fn get_api_key(
    pg: &PostgresPool,
    salt: &str,
    req: &ServiceRequest,
) -> Result<Option<super::AuthenticationInfo>> {
    if let Ok(query) = actix_web::web::Query::<ApiQueryString>::from_query(req.query_string()) {
        let auth = handle_api_key(pg, salt, &query.0.api_key).await?;
        return Ok(Some(auth));
    }

    if let Ok(header) = Authorization::<Bearer>::parse(req) {
        let key = header.into_scheme();
        let auth = handle_api_key(pg, salt, key.token()).await?;
        return Ok(Some(auth));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::KeyAndHash;
    use crate::error::Result;
    use assert_matches::assert_matches;

    #[test]
    fn valid_key() -> Result<()> {
        let k = KeyAndHash::new("s");
        assert_eq!(KeyAndHash::from_key("s", &k.key)?, (k.api_key_id, k.hash));
        Ok(())
    }

    #[test]
    fn bad_salt() -> Result<()> {
        let key = KeyAndHash::new("12345");
        let result = KeyAndHash::from_key("abc", &key.key)?;
        assert_eq!(key.api_key_id, result.0);
        assert_ne!(
            key.hash, result.1,
            "hash with different salt should not match"
        );
        Ok(())
    }

    #[test]
    fn bad_key() -> Result<()> {
        let key = KeyAndHash::new("12345");

        let mut bad_key = String::from(key.key);
        bad_key.pop();
        bad_key.push('a');

        let result = KeyAndHash::from_key("12345", &bad_key)?;
        assert_eq!(key.api_key_id, result.0);
        assert_ne!(
            key.hash, result.1,
            "hash with different key should not match"
        );
        Ok(())
    }

    #[test]
    fn bad_prefix() {
        let key = KeyAndHash::new("12345");
        let bad_key = format!("aa1.{}", key.key.chars().skip(4).collect::<String>());
        KeyAndHash::from_key("12345", &bad_key).expect_err("bad prefix");
    }

    #[test]
    fn bad_length() {
        let key = KeyAndHash::new("12345");
        let mut bad_key = String::from(key.key);
        bad_key.push('a');

        KeyAndHash::from_key("12345", &bad_key).expect_err("length too high");

        bad_key.pop();
        bad_key.pop();
        KeyAndHash::from_key("12345", &bad_key).expect_err("length too low");
    }
}
