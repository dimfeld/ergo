use crate::{
    database::PostgresPool,
    error::{Error, Result},
};
use actix_identity::Identity;
use actix_web::HttpRequest;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sqlx::{postgres::PgRow, query, query::Query, Encode, FromRow, Postgres};
use uuid::Uuid;

pub mod handlers;
pub mod middleware;

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "permission")]
#[sqlx(rename_all = "snake_case")]
pub enum PermissionType {
    #[serde(rename = "trigger_event")]
    TriggerEvent,
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "write")]
    Write,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Permission {
    #[serde(rename = "permission_type")]
    pub perm: PermissionType,
    #[serde(rename = "permissioned_object")]
    pub object: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKey {
    api_key_id: Uuid,
    prefix: String,
    org_id: Uuid,
    user_id: Option<Uuid>,
    inherits_user_permissions: bool,
    description: Option<String>,
    active: bool,
    expires: Option<DateTime<Utc>>,
    created: DateTime<Utc>,
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
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub email: String,
    pub user_entity_ids: UserEntityList,
}

#[derive(Debug, Clone)]
pub enum Authenticated {
    ApiKey {
        key: ApiKey,
        user: Option<RequestUser>,
    },
    User(RequestUser),
}

pub type UserEntityList = smallvec::SmallVec<[Uuid; 4]>;

impl Authenticated {
    pub fn org_id(&self) -> &Uuid {
        match self {
            Self::User(user) => &user.org_id,
            Self::ApiKey { key, .. } => &key.org_id,
        }
    }

    pub fn user_entity_ids(&self) -> UserEntityList {
        match self {
            Self::User(user) => user.user_entity_ids.clone(),
            Self::ApiKey { key, user } => match (key.inherits_user_permissions, user) {
                (false, _) => {
                    let mut list = UserEntityList::new();
                    list.push(key.api_key_id.clone());
                    list
                }
                (true, Some(user)) => user.user_entity_ids.clone(),
                (true, None) => {
                    let mut list = UserEntityList::new();
                    list.push(key.api_key_id.clone());
                    list.push(key.org_id.clone());
                    list
                }
            },
        }
    }
}

async fn get_api_key(pg: &PostgresPool, req: &HttpRequest) -> Result<Option<Authenticated>> {
    // Extract key from headers of query string.
    // Hash the provided key
    // Match the key against the
    Ok(None)
}

async fn get_user_info(pg: &PostgresPool, user_id: &Uuid) -> Result<RequestUser> {
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
            Some(roles) => {
                let mut ids = UserEntityList::from_vec(roles);
                ids.push(user.user_id);
                ids
            }
            None => UserEntityList::from_elem(user.user_id, 1),
        };

        RequestUser {
            user_id: user.user_id,
            org_id: user.org_id,
            name: user.name,
            email: user.email,
            user_entity_ids,
        }
    })
    .ok_or(Error::AuthenticationError)
}

// Authenticate via cookie or API key, depending on what's provided.
pub async fn authenticate(
    pg: &PostgresPool,
    identity: &Identity,
    req: &HttpRequest,
) -> Result<Authenticated> {
    if let Some(auth) = get_api_key(pg, req).await? {
        return Ok(auth);
    }

    let user_id = identity
        .identity()
        .ok_or(Error::AuthenticationError)
        .and_then(|s| Uuid::parse_str(&s).map_err(Error::from))?;

    let req_user = get_user_info(pg, &user_id).await?;
    Ok(Authenticated::User(req_user))
}

pub async fn authenticate_request_user(
    pg: &PostgresPool,
    identity: &Identity,
    req: &HttpRequest,
) -> Result<RequestUser> {
    let auth = authenticate(pg, identity, req).await?;
    match auth {
        Authenticated::User(user) => Ok(user),
        Authenticated::ApiKey {
            user: Some(user), ..
        } => Ok(user),
        Authenticated::ApiKey { key, .. } => {
            let user_id = key.user_id.as_ref().ok_or(Error::AuthenticationError)?;
            get_user_info(&pg, user_id).await
        }
    }
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
        FROM {object_table} obj
        JOIN user_entity_permissions ON user_entity_id = $2 AND permission_type = $4 AND permissioned_object IN (1, $3)
        WHERE obj.org_id = $1 AND obj.{object_id_column} = $3
    )"##,
        object_table = object_table,
        object_id_column = object_id_column
    );

    let q = sqlx::query(&query_str);
    let q = match user {
        Authenticated::User(user) => q.bind(user.org_id).bind(user.user_id),
        Authenticated::ApiKey { key, .. } => q.bind(key.org_id).bind(key.api_key_id),
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
