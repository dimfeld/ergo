use crate::{database::PostgresPool, error::Error};
use actix_identity::Identity;
use actix_web::HttpRequest;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, query, Encode, FromRow, Postgres};
use uuid::Uuid;

pub mod handlers;
pub mod middleware;

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "permission")]
#[sqlx(rename_all = "snake_case")]
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
        key: Uuid,
        org_id: Uuid,
        user_id: Option<Uuid>,
        expires: Option<DateTime<Utc>>,
    },
}

impl ApiKeyToken {
    pub fn key(&self) -> &Uuid {
        match self {
            ApiKeyToken::V0 { key, .. } => key,
        }
    }

    pub fn user_id(&self) -> Option<&Uuid> {
        match self {
            ApiKeyToken::V0 { user_id, .. } => user_id.as_ref(),
        }
    }

    pub fn org_id(&self) -> &Uuid {
        match self {
            ApiKeyToken::V0 { org_id, .. } => org_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKey {
    pub api_key_id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
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
    pub user_id: Uuid,
    pub external_user_id: String,
    pub active_org_id: Uuid,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct RequestUser {
    user_id: Uuid,
    org_id: Uuid,
    name: String,
    email: String,
    user_entity_ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub enum Authenticated {
    ApiKey(ApiKeyToken),
    User(RequestUser),
}

impl Authenticated {
    pub fn org_and_user(&self) -> (&Uuid, &Uuid) {
        match self {
            Authenticated::ApiKey(key) => (key.org_id(), key.key()),
            Authenticated::User(user) => (&user.org_id, &user.user_id),
        }
    }
}

fn get_api_key(req: &HttpRequest) -> Result<Option<Authenticated>, Error> {
    Ok(None)
}

// Authenticate via cookie or json web token, depending on what's provided.
pub async fn authenticate(
    pg: &PostgresPool,
    identity: &Identity,
    req: &HttpRequest,
) -> Result<Authenticated, Error> {
    if let Some(auth) = get_api_key(req)? {
        return Ok(auth);
    }

    let user_id = identity
        .identity()
        .ok_or(Error::AuthenticationError)
        .and_then(|s| Uuid::parse_str(&s).map_err(Error::from))?;

    query!(
        r##"SELECT user_id,
            active_org_id AS org_id, users.name, email,
            array_agg(role_id) AS roles
        FROM users
        JOIN orgs ON orgs.org_id = active_org_id
        LEFT JOIN user_roles USING(user_id, org_id)
        WHERE user_id = $1 AND users.active AND orgs.active
        GROUP BY user_id"##,
        user_id
    )
    .fetch_optional(pg)
    .await?
    .map(|user| {
        let user_entity_ids = match user.roles {
            Some(mut roles) => {
                roles.push(user.user_id);
                roles
            }
            None => vec![user.user_id],
        };

        let req_user = RequestUser {
            user_id: user.user_id,
            org_id: user.org_id,
            name: user.name,
            email: user.email,
            user_entity_ids,
        };

        Authenticated::User(req_user)
    })
    .ok_or(Error::AuthenticationError)
}

pub async fn get_permitted_object<T, ID>(
    pool: &PostgresPool,
    user: &Authenticated,
    object_table: &str,
    object_id_column: &str,
    permission: PermissionType,
    object_id: ID,
) -> Result<T, Error>
where
    T: Send + Unpin + for<'r> FromRow<'r, PgRow>,
    ID: Send + Unpin + for<'r> Encode<'r, Postgres> + sqlx::Type<Postgres>,
{
    let query_str = format!(
        r##"SELECT obj.* as match
        FROM {object_table} obj ON obj.org_id = $1 AND obj.{object_id_column} = $3
        JOIN user_entity_permissions ON user_entity_id = $2 AND permission_type = $4 AND permissioned_object IN (1, $3)
    )"##,
        object_table = object_table,
        object_id_column = object_id_column
    );

    let q = sqlx::query(&query_str);

    let q = match user {
        Authenticated::ApiKey(key) => q.bind(key.org_id()).bind(key.key()),
        Authenticated::User(user) => q.bind(user.org_id).bind(user.user_id),
    };

    let row = q
        .bind(object_id)
        .bind(permission)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = row {
        Ok(T::from_row(&row)?)
    } else {
        Err(Error::AuthorizationError)
    }
}
