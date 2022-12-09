pub mod api_key;
pub mod error;
pub mod middleware;
pub mod password;

pub use error::*;

use std::{
    future::{ready, Ready},
    rc::Rc,
    str::FromStr,
};

pub use api_key::ApiKey;

use actix_web::{dev::ServiceRequest, FromRequest, HttpMessage, HttpRequest};
use chrono::{DateTime, Utc};
use ergo_database::{
    object_id::{OrgId, RoleId, UserId},
    PostgresPool,
};
use serde::{Deserialize, Serialize};
use sqlx::{query, PgConnection};
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
    pub object: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub user_id: UserId,
    pub external_user_id: String,
    pub active_org_id: OrgId,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RequestUser {
    pub user_id: UserId,
    pub org_id: OrgId,
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

    pub fn expect_authed(self) -> Result<Rc<AuthenticationInfo>, Error> {
        self.0.ok_or(Error::AuthenticationError)
    }
}

impl FromRequest for MaybeAuthenticated {
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
        user: RequestUser,
    },
    User(RequestUser),
}

pub type UserEntityList = smallvec::SmallVec<[Uuid; 4]>;

impl AuthenticationInfo {
    pub fn org_id(&self) -> &OrgId {
        match self {
            Self::User(user) => &user.org_id,
            Self::ApiKey { key, .. } => &key.org_id,
        }
    }

    pub fn user_id(&self) -> &UserId {
        match self {
            Self::User(user) => &user.user_id,
            Self::ApiKey { user, .. } => &user.user_id,
        }
    }

    pub fn user_entity_ids(&self) -> UserEntityList {
        match self {
            Self::User(user) => user.user_entity_ids.clone(),
            Self::ApiKey { key, user } => match (key.inherits_user_permissions, user) {
                (false, _) => {
                    let mut list = UserEntityList::new();
                    list.push(key.api_key_id);
                    list
                }
                (true, user) => {
                    let mut ids = user.user_entity_ids.clone();
                    ids.push(key.api_key_id);
                    ids
                }
            },
        }
    }

    pub fn expect_admin(&self) -> Result<(), Error> {
        let is_admin = match self {
            Self::User(user) => user.is_admin,
            Self::ApiKey { user, .. } => user.is_admin,
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
    admin_user: Option<UserId>,
}

impl AuthData {
    pub fn new(pg_pool: PostgresPool) -> Result<AuthData, Error> {
        Ok(AuthData {
            pg: pg_pool,
            admin_user: envoption::optional("ADMIN_USER_ID")?,
        })
    }

    // Authenticate via cookie or API key, depending on what's provided.
    pub async fn authenticate(
        &self,
        identity: Option<actix_identity::Identity>,
        req: &ServiceRequest,
    ) -> Result<Option<AuthenticationInfo>, Error> {
        if let Some(auth) = api_key::get_api_key(self, req).await? {
            return Ok(Some(auth));
        }

        match identity {
            Some(identity) => {
                // TODO This should be a session ID, not a user ID.
                let user_id =
                    UserId::from_str(&identity.id().map_err(|_| Error::AuthenticationError)?)
                        .map_err(|_| Error::AuthenticationError)?;

                let req_user = self.get_user_info(&user_id).await?;
                Ok(Some(AuthenticationInfo::User(req_user)))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(user))]
    async fn get_user_info(&self, user_id: &UserId) -> Result<RequestUser, Error> {
        let mut conn = self.pg.acquire().await?;
        get_user_info(&mut conn, user_id, self.admin_user.as_ref()).await
    }
}

pub async fn get_user_info(
    tx: &mut PgConnection,
    user_id: &UserId,
    admin_user: Option<&UserId>,
) -> Result<RequestUser, Error> {
    event!(Level::DEBUG, "Fetching user");
    query!(
        r##"SELECT user_id as "user_id: UserId",
            active_org_id AS "org_id: OrgId", users.name, email,
            array_agg(role_id) FILTER(WHERE role_id IS NOT NULL) AS "roles: Vec<RoleId>"
        FROM users
        JOIN orgs ON orgs.org_id = active_org_id
        LEFT JOIN user_roles USING(user_id, org_id)
        WHERE user_id = $1 AND NOT users.deleted AND NOT orgs.deleted
        GROUP BY user_id"##,
        &user_id.0
    )
    .fetch_optional(tx)
    .await?
    .map(|user| {
        let user_entity_ids = match user.roles {
            Some(roles) => {
                let mut ids = UserEntityList::with_capacity(roles.len() + 1);
                for role in roles {
                    ids.push(role.into());
                }
                ids.push(user.user_id.clone().into());
                ids
            }
            None => UserEntityList::from_elem(user.user_id.clone().into(), 1),
        };

        let user = RequestUser {
            user_id: user.user_id,
            org_id: user.org_id,
            name: user.name,
            email: user.email,
            user_entity_ids,
            is_admin: admin_user.map(|u| u == user_id).unwrap_or(false),
        };

        tracing::Span::current().record("user", &field::debug(&user));

        user
    })
    .ok_or(Error::AuthenticationError)
}

#[cfg(test)]
mod tests {
    use super::{api_key::ApiKeyAuth, *};
    mod authentication_info {
        use ergo_database::object_id::*;
        use smallvec::smallvec;
        use std::str::FromStr;
        use uuid::Uuid;

        use super::*;

        fn user_id() -> UserId {
            UserId::from_uuid(Uuid::from_str("e1ecedb3-10a5-4fa5-ae8d-edb383aac701").unwrap())
        }

        fn org_id() -> OrgId {
            OrgId::from_uuid(Uuid::from_str("622217aa-6a58-45ea-812f-749b8ad462bf").unwrap())
        }

        fn api_key_id() -> Uuid {
            Uuid::from_str("353d3c01-d0ea-46ea-95e4-3a07d0ce9116").unwrap()
        }

        fn role_id() -> RoleId {
            RoleId::from_uuid(Uuid::from_str("27849616-d3a4-43c6-995d-143cf1c8de98").unwrap())
        }

        fn request_user() -> RequestUser {
            let user = user_id();
            let org = org_id();
            let ids = smallvec![user.0, org.0, role_id().0];
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
                    user_id: user.user_id.clone(),
                    inherits_user_permissions: true,
                },
                user,
            }
        }

        fn user_key_without_inherit() -> AuthenticationInfo {
            let user = request_user();
            AuthenticationInfo::ApiKey {
                key: ApiKeyAuth {
                    api_key_id: api_key_id(),
                    org_id: user.org_id.clone(),
                    user_id: user.user_id.clone(),
                    inherits_user_permissions: false,
                },
                user,
            }
        }

        fn user_auth() -> AuthenticationInfo {
            AuthenticationInfo::User(request_user())
        }

        #[test]
        fn get_user_id() {
            assert_eq!(
                user_key_with_inherit().user_id(),
                &user_id(),
                "key with inherit"
            );
            assert_eq!(
                user_key_without_inherit().user_id(),
                &user_id(),
                "key without inherit"
            );
            assert_eq!(user_auth().user_id(), &user_id(), "user auth");
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
            assert_eq!(user_auth().org_id(), &org_id(), "user auth");
        }

        #[test]
        fn user_entity_ids() {
            let mut ids = user_key_with_inherit().user_entity_ids();
            ids.sort();
            let mut s: UserEntityList = smallvec![
                user_id().into(),
                org_id().into(),
                api_key_id(),
                role_id().into()
            ];
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

            let mut ids = user_auth().user_entity_ids();
            ids.sort();
            let mut s: UserEntityList =
                smallvec![org_id().into(), user_id().into(), role_id().into()];
            s.sort();
            assert_eq!(ids, s, "user auth should have user, org, and roles");
        }
    }
}
