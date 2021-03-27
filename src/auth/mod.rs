use crate::error::Error;
use crate::pool;
use crate::vault::VaultPostgresPool;
use actix_session::Session;
use actix_web::{cookie::Cookie, HttpMessage, HttpRequest};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::query;

pub mod handlers;
pub mod middleware;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PermissionType {
    #[serde(rename = "trigger_event")]
    TriggerEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Permission {
    #[serde(rename = "permission_type")]
    pub perm: PermissionType,
    #[serde(rename = "permissioned_object")]
    pub object: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "v")]
pub enum ApiKeyToken {
    #[serde(rename = "0")]
    V0 {
        key: String,
        org_id: i64,
        user_id: Option<i64>,
        expires: Option<DateTime<Utc>>,
        permissions: Vec<Permission>,
    },
}

impl ApiKeyToken {
    pub fn permissions(&self) -> &[Permission] {
        match self {
            ApiKeyToken::V0 { permissions, .. } => permissions.as_slice(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKey {
    pub api_key: String,
    secret_key_hash: String,
    pub user_entity_id: i32,
    pub description: Option<String>,
    pub active: bool,
    pub expires: Option<DateTime<Utc>>,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyPermission {
    pub api_key: String,
    #[serde(flatten)]
    pub permission: Permission,
}

#[derive(Debug, Clone)]
pub struct User {
    pub user_id: i32,
    pub external_user_id: String,
    pub active_org_id: i32,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum Authenticated {
    ApiKey(ApiKeyToken),
    User(User),
}

fn get_api_key(req: &HttpRequest) -> Result<Option<Authenticated>, Error> {
    Ok(None)
}

// Authenticate via cookie or json web token, depending on what's provided.
pub async fn authenticate(
    pg: &VaultPostgresPool<()>,
    session: &Session,
    req: &HttpRequest,
) -> Result<Authenticated, Error> {
    if let Some(auth) = get_api_key(req)? {
        return Ok(auth);
    }

    let session_id: i64 = session.get("sid")?.ok_or(Error::AuthenticationError)?;
    let user_data = query!(r##""##).execute(pool!(pg)).await;

    Err(Error::AuthenticationError)
}

pub async fn check_object_permission(
    pool: &VaultPostgresPool<()>,
    user: &Authenticated,
    permissions: &[Permission],
) -> Result<bool, Error> {
    Ok(true)
}

pub async fn authenticate_for_permission(
    pool: &VaultPostgresPool<()>,
    permission: Permission,
    req: &HttpRequest,
) -> Result<Authenticated, Error> {
    if let Some(auth) = get_api_key(req)? {
        let permitted = check_object_permission(pool, &auth, &[permission]).await?;
        if permitted {
            return Ok(auth);
        } else {
            return Err(Error::AuthorizationError);
        }
    }

    Err(Error::AuthenticationError)
}
