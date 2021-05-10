use crate::{
    database::PostgresPool,
    error::{Error, Result},
};
use actix_web::{dev::ServiceRequest, http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::Digest;
use sqlx::{postgres::PgRow, query, query::Query, Encode, FromRow, Postgres};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use super::AuthData;

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

pub struct ApiKeyData {
    pub api_key_id: Uuid,
    pub key: String,
    pub hash: Vec<u8>,
}

impl ApiKeyData {
    pub fn new() -> ApiKeyData {
        let id = Uuid::new_v4();
        let base64_id = base64::encode_config(id.as_bytes(), base64::URL_SAFE_NO_PAD);
        let random = base64::encode_config(Uuid::new_v4().as_bytes(), base64::URL_SAFE_NO_PAD);
        let key = format!("er1.{}.{}", base64_id, random);
        let hash = hash_key(&key);

        ApiKeyData {
            api_key_id: id,
            key,
            hash,
        }
    }
}

fn hash_key(key: &str) -> Vec<u8> {
    let mut hasher = sha3::Sha3_512::default();
    hasher.update(key.as_bytes());
    hasher.finalize().to_vec()
}

fn decode_key(key: &str) -> Result<(Uuid, Vec<u8>)> {
    if !key.starts_with("er1.") || key.len() != 49 {
        return Err(Error::AuthenticationError);
    }

    let hash = hash_key(key);
    let id_portion = key
        .split('.')
        .skip(1)
        .next()
        .ok_or(Error::AuthenticationError)?;
    let api_key_bytes = base64::decode_config(id_portion.as_bytes(), base64::URL_SAFE_NO_PAD)
        .map_err(|_| Error::AuthenticationError)?;
    let api_key_id = Uuid::from_slice(&api_key_bytes).map_err(|_| Error::AuthenticationError)?;

    Ok((api_key_id, hash))
}

#[derive(Deserialize)]
struct ApiQueryString {
    api_key: String,
}

async fn handle_api_key(auth_data: &AuthData, key: &str) -> Result<super::AuthenticationInfo> {
    let (api_key_id, hash) = decode_key(key)?;
    let auth_key = sqlx::query_as!(
        ApiKeyAuth,
        "SELECT api_key_id, org_id, user_id, inherits_user_permissions
        FROM api_keys
        WHERE api_key_id=$1 AND hash=$2 AND active AND (expires IS NULL OR expires < now())
        LIMIT 1",
        api_key_id,
        hash
    )
    .fetch_optional(&auth_data.pg)
    .await?
    .ok_or_else(|| Error::AuthenticationError)?;

    let user = match &auth_key.user_id {
        None => None,
        // This could be combined with the query above, but for simplicity we just keep it separate
        // for now.
        Some(id) => Some(auth_data.get_user_info(id).await?),
    };

    Ok(super::AuthenticationInfo::ApiKey {
        key: auth_key,
        user,
    })
}

#[instrument(level = "DEBUG", skip(auth_data))]
pub async fn get_api_key(
    auth_data: &AuthData,
    req: &ServiceRequest,
) -> Result<Option<super::AuthenticationInfo>> {
    event!(Level::DEBUG, "Fetching api key");
    if let Ok(query) = actix_web::web::Query::<ApiQueryString>::from_query(req.query_string()) {
        event!(Level::DEBUG, key=%query.0.api_key, "Got key from query string");
        let auth = handle_api_key(auth_data, &query.0.api_key).await?;
        return Ok(Some(auth));
    }

    if let Ok(header) = Authorization::<Bearer>::parse(req) {
        event!(Level::DEBUG, key=%header, "Got key from auth header");
        let key = header.into_scheme();
        let auth = handle_api_key(auth_data, key.token()).await?;
        return Ok(Some(auth));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{decode_key, ApiKeyData};
    use crate::error::Result;
    use assert_matches::assert_matches;

    #[test]
    fn valid_key() -> Result<()> {
        let data = ApiKeyData::new();

        let (api_key_id, hash) = decode_key(&data.key)?;
        assert_eq!(api_key_id, data.api_key_id, "api_key_id");
        assert_eq!(hash, data.hash, "hash");
        Ok(())
    }

    #[test]
    fn bad_key() -> Result<()> {
        let data = ApiKeyData::new();

        // Alter the key.
        let mut key = String::from(data.key);
        key.pop();
        key.push('a');

        let (api_key_id, hash) = decode_key(&key)?;
        assert_eq!(api_key_id, data.api_key_id, "api_key_id");
        assert_ne!(hash, data.hash, "hash");
        Ok(())
    }

    #[test]
    fn bad_prefix() {
        let data = ApiKeyData::new();
        let bad_key = format!("aa1.{}", data.key.chars().skip(4).collect::<String>());
        decode_key(&bad_key).expect_err("bad prefix");
    }

    #[test]
    fn bad_length() {
        let data = ApiKeyData::new();

        let mut key = String::from(&data.key);
        key.push('a');
        decode_key(&key).expect_err("length too high");

        key.pop();
        key.pop();
        decode_key(&key).expect_err("length too low");
    }
}
