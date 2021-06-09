pub mod api_key;
pub mod handlers;
pub mod middleware;
pub mod password;

use std::{
    future::{ready, Ready},
    rc::Rc,
};

pub use api_key::ApiKey;

use crate::{
    database::PostgresPool,
    error::{Error, Result},
};
use actix_web::{dev::ServiceRequest, FromRequest, HttpRequest};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::query;
use tracing::{event, field, instrument, Level};
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
pub struct MaybeAuthenticated(Option<Rc<AuthenticationInfo>>);

impl MaybeAuthenticated {
    pub fn into_inner(self) -> Option<Rc<AuthenticationInfo>> {
        self.0
    }

    pub fn expect_authed(self) -> Result<Rc<AuthenticationInfo>> {
        self.0.ok_or(Error::AuthenticationError)
    }
}

impl FromRequest for MaybeAuthenticated {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let value = req.extensions().get::<Rc<AuthenticationInfo>>().cloned();
        ready(Ok(MaybeAuthenticated(value)))
    }
}

impl std::ops::Deref for MaybeAuthenticated {
    type Target = Option<Rc<AuthenticationInfo>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extracts authentication information for routes that must be authenticated.
/// Returns an Error::AuthenticationError if the user is not authenticated.
#[derive(Debug)]
pub struct Authenticated(Rc<AuthenticationInfo>);

impl Authenticated {
    pub fn into_inner(self) -> Rc<AuthenticationInfo> {
        self.0
    }
}

impl FromRequest for Authenticated {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let value = req.extensions().get::<Rc<AuthenticationInfo>>().cloned();
        let result = match value {
            Some(v) => Ok(Authenticated(v)),
            None => Err(Error::AuthenticationError),
        };
        ready(result)
    }
}

impl std::ops::Deref for Authenticated {
    type Target = Rc<AuthenticationInfo>;

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

    pub fn user_id(&self) -> Option<&Uuid> {
        match self {
            Self::User(user) => Some(&user.user_id),
            Self::ApiKey { user, .. } => user.as_ref().map(|u| &u.user_id),
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
                (true, Some(user)) => {
                    let mut ids = user.user_entity_ids.clone();
                    ids.push(key.api_key_id.clone());
                    ids
                }
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
        identity: Option<String>,
        req: &ServiceRequest,
    ) -> Result<Option<AuthenticationInfo>> {
        if let Some(auth) = api_key::get_api_key(self, req).await? {
            return Ok(Some(auth));
        }

        match identity {
            Some(identity) => {
                // TODO This should be a session ID, not a user ID.
                let user_id = Uuid::parse_str(&identity)?;

                let req_user = self.get_user_info(&user_id).await?;
                Ok(Some(AuthenticationInfo::User(req_user)))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(user))]
    async fn get_user_info(&self, user_id: &Uuid) -> Result<RequestUser> {
        event!(Level::DEBUG, "Fetching user");
        query!(
            r##"SELECT user_id,
            active_org_id AS org_id, users.name, email,
            array_agg(role_id) FILTER(WHERE role_id IS NOT NULL) AS roles
        FROM users
        JOIN orgs ON orgs.org_id = active_org_id
        LEFT JOIN user_roles USING(user_id, org_id)
        WHERE user_id = $1 AND NOT users.deleted AND NOT orgs.deleted
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

            let user = RequestUser {
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
            };

            tracing::Span::current().record("user", &field::debug(&user));

            user
        })
        .ok_or(Error::AuthenticationError)
    }
}

#[cfg(test)]
mod tests {
    mod authentication_info {
        use smallvec::smallvec;
        use std::str::FromStr;
        use uuid::Uuid;

        use crate::auth::{api_key::ApiKeyAuth, AuthenticationInfo, RequestUser, UserEntityList};

        fn user_id() -> Uuid {
            Uuid::from_str("e1ecedb3-10a5-4fa5-ae8d-edb383aac701").unwrap()
        }

        fn org_id() -> Uuid {
            Uuid::from_str("622217aa-6a58-45ea-812f-749b8ad462bf").unwrap()
        }

        fn api_key_id() -> Uuid {
            Uuid::from_str("353d3c01-d0ea-46ea-95e4-3a07d0ce9116").unwrap()
        }

        fn role_id() -> Uuid {
            Uuid::from_str("27849616-d3a4-43c6-995d-143cf1c8de98").unwrap()
        }

        fn request_user() -> RequestUser {
            let user = user_id();
            let org = org_id();
            let ids = smallvec![user.clone(), org.clone(), role_id()];
            RequestUser {
                user_id: user,
                org_id: org,
                user_entity_ids: ids,
                is_admin: false,
                email: "a@example.com".to_string(),
                name: "Test User".to_string(),
            }
        }

        fn user_key_with_inherit() -> AuthenticationInfo {
            let user = request_user();
            AuthenticationInfo::ApiKey {
                key: ApiKeyAuth {
                    api_key_id: api_key_id(),
                    org_id: user.org_id.clone(),
                    user_id: Some(user.user_id.clone()),
                    inherits_user_permissions: true,
                },
                user: Some(user),
            }
        }

        fn user_key_without_inherit() -> AuthenticationInfo {
            let user = request_user();
            AuthenticationInfo::ApiKey {
                key: ApiKeyAuth {
                    api_key_id: api_key_id(),
                    org_id: user.org_id.clone(),
                    user_id: Some(user.user_id.clone()),
                    inherits_user_permissions: false,
                },
                user: Some(user),
            }
        }

        fn org_key_with_inherit() -> AuthenticationInfo {
            AuthenticationInfo::ApiKey {
                user: None,
                key: ApiKeyAuth {
                    api_key_id: api_key_id(),
                    org_id: org_id(),
                    user_id: None,
                    inherits_user_permissions: true,
                },
            }
        }

        fn org_key_without_inherit() -> AuthenticationInfo {
            AuthenticationInfo::ApiKey {
                user: None,
                key: ApiKeyAuth {
                    api_key_id: api_key_id(),
                    org_id: org_id(),
                    user_id: None,
                    inherits_user_permissions: false,
                },
            }
        }

        fn user_auth() -> AuthenticationInfo {
            AuthenticationInfo::User(request_user())
        }

        #[test]
        fn get_user_id() {
            assert_eq!(
                user_key_with_inherit().user_id(),
                Some(&user_id()),
                "key with inherit"
            );
            assert_eq!(
                user_key_without_inherit().user_id(),
                Some(&user_id()),
                "key without inherit"
            );
            assert_eq!(
                org_key_with_inherit().user_id(),
                None,
                "org key with inherit"
            );
            assert_eq!(
                org_key_without_inherit().user_id(),
                None,
                "org key without inherit"
            );
            assert_eq!(user_auth().user_id(), Some(&user_id()), "user auth");
        }

        #[test]
        fn get_org_id() {
            assert_eq!(
                user_key_with_inherit().org_id(),
                &org_id(),
                "key with inherit"
            );
            assert_eq!(
                user_key_without_inherit().org_id(),
                &org_id(),
                "key without inherit"
            );
            assert_eq!(
                org_key_with_inherit().org_id(),
                &org_id(),
                "org key with inherit"
            );
            assert_eq!(
                org_key_without_inherit().org_id(),
                &org_id(),
                "org key without inherit"
            );
            assert_eq!(user_auth().org_id(), &org_id(), "user auth");
        }

        #[test]
        fn user_entity_ids() {
            let mut ids = user_key_with_inherit().user_entity_ids();
            ids.sort();
            let mut s: UserEntityList = smallvec![user_id(), org_id(), api_key_id(), role_id()];
            s.sort();
            assert_eq!(
                ids, s,
                "key with inherit should have API key, user, org, and role"
            );

            let mut ids = user_key_without_inherit().user_entity_ids();
            ids.sort();
            let mut s: UserEntityList = smallvec![api_key_id()];
            s.sort();
            assert_eq!(ids, s, "key without inherit should only have API key");

            let mut ids = org_key_with_inherit().user_entity_ids();
            ids.sort();
            let mut s: UserEntityList = smallvec![org_id(), api_key_id()];
            s.sort();
            assert_eq!(ids, s, "org key with inherit should have API key and org");

            let mut ids = org_key_without_inherit().user_entity_ids();
            ids.sort();
            let mut s: UserEntityList = smallvec![api_key_id()];
            s.sort();
            assert_eq!(
                ids, s,
                "org key without inherit should have API key and org"
            );

            let mut ids = user_auth().user_entity_ids();
            ids.sort();
            let mut s: UserEntityList = smallvec![org_id(), user_id(), role_id()];
            s.sort();
            assert_eq!(ids, s, "user auth should have user, org, and roles");
        }
    }
}
