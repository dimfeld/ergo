use crate::{
    database::PostgresPool,
    error::{Error, Result},
};
use actix_web::HttpRequest;
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

pub async fn get_api_key(
    pg: &PostgresPool,
    req: &HttpRequest,
) -> Result<Option<super::Authenticated>> {
    // Extract key from headers of query string.
    // Hash the provided key
    // Match the key against the
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
