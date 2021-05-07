pub mod api_key;
pub mod handlers;
pub mod middleware;
pub mod password;

use std::{
    future::{ready, Ready},
    sync::Arc,
};

use api_key::get_api_key;
pub use api_key::ApiKey;

use crate::{
    database::PostgresPool,
    error::{Error, Result},
};
use actix_identity::Identity;
use actix_web::{dev::ServiceRequest, FromRequest, HttpRequest};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, query, query::Query, Encode, FromRow, Postgres};
use uuid::Uuid;

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
    pub is_admin: bool,
}

/// Extracts authentication information for routes that optionally require it.
pub struct MaybeAuthenticated(Option<Arc<AuthenticationInfo>>);

impl MaybeAuthenticated {
    pub fn into_inner(self) -> Option<Arc<AuthenticationInfo>> {
        self.0
    }

    pub fn expect_authed(self) -> Result<Arc<AuthenticationInfo>> {
        self.0.ok_or(Error::AuthenticationError)
    }
}

impl FromRequest for MaybeAuthenticated {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        let value = req.extensions().get::<Arc<AuthenticationInfo>>().cloned();
        ready(Ok(MaybeAuthenticated(value)))
    }
}

impl std::ops::Deref for MaybeAuthenticated {
    type Target = Option<Arc<AuthenticationInfo>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extracts authentication information for routes that must be authenticated.
/// Returns an Error::AuthenticationError if the user is not authenticated.
pub struct Authenticated(Arc<AuthenticationInfo>);

impl Authenticated {
    pub fn into_inner(self) -> Arc<AuthenticationInfo> {
        self.0
    }
}

impl FromRequest for Authenticated {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        let value = req.extensions().get::<Arc<AuthenticationInfo>>().cloned();
        let result = match value {
            Some(v) => Ok(Authenticated(v)),
            None => Err(Error::AuthenticationError),
        };
        ready(result)
    }
}

impl std::ops::Deref for Authenticated {
    type Target = Arc<AuthenticationInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub enum AuthenticationInfo {
    ApiKey {
        key: api_key::ApiKeyAuth,
        user: Option<RequestUser>,
    },
    User(RequestUser),
}

pub type UserEntityList = smallvec::SmallVec<[Uuid; 4]>;

impl AuthenticationInfo {
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

    pub fn expect_admin(&self) -> Result<()> {
        let is_admin = match self {
            Self::User(user) => user.is_admin,
            Self::ApiKey { user, .. } => user.as_ref().map(|u| u.is_admin).unwrap_or(false),
        };

        if is_admin {
            Ok(())
        } else {
            Err(Error::AuthorizationError)
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuthData {
    pg: PostgresPool,
    /// Temporary method of implementing admin user
    admin_user: Option<Uuid>,
}

impl AuthData {
    pub fn new(pg_pool: PostgresPool) -> Result<AuthData> {
        Ok(AuthData {
            pg: pg_pool,
            admin_user: envoption::optional("ADMIN_USER_ID")?,
        })
    }

    // Authenticate via cookie or API key, depending on what's provided.
    pub async fn authenticate(
        &self,
        identity: &str,
        req: &ServiceRequest,
    ) -> Result<AuthenticationInfo> {
        if let Some(auth) = api_key::get_api_key(self, req).await? {
            return Ok(auth);
        }

        let user_id = Uuid::parse_str(identity)?;

        let req_user = self.get_user_info(&user_id).await?;
        Ok(AuthenticationInfo::User(req_user))
    }

    async fn get_user_info(&self, user_id: &Uuid) -> Result<RequestUser> {
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
        .fetch_optional(&self.pg)
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
                is_admin: self
                    .admin_user
                    .as_ref()
                    .map(|u| u == user_id)
                    .unwrap_or(false),
            }
        })
        .ok_or(Error::AuthenticationError)
    }
}

pub async fn get_permitted_object<T, ID>(
    pool: &PostgresPool,
    user: &AuthenticationInfo,
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
        AuthenticationInfo::User(user) => q.bind(user.org_id).bind(user.user_id),
        AuthenticationInfo::ApiKey { key, .. } => q.bind(key.org_id).bind(key.api_key_id),
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
