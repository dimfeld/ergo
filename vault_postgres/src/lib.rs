use async_trait::async_trait;
use deadpool::managed::{Pool, RecycleError};
use hashicorp_vault::client::{PostgresqlLogin, VaultClient};
use sqlx::{Connection, PgConnection};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SqlError(#[from] sqlx::error::Error),

    #[error("Vault Error")]
    VaultError(#[from] hashicorp_vault::Error),

    #[error("Vault returned no auth data")]
    VaultNoDataError,
}

pub struct VaultPostgresPool {
    vault_client: Arc<RwLock<VaultClient<()>>>,
    pool: Pool<WrappedConnection, Error>,
    manager: Arc<Manager>,

    host: String,
    database: String,
    role: String,
}

impl VaultPostgresPool {
    fn new(
        max_connections: usize,
        host: String,
        database: String,
        role: String,
        vault_client: Arc<RwLock<VaultClient<()>>>,
    ) -> Result<Arc<VaultPostgresPool>, Error> {
        let manager = Arc::new(Manager {
            connection_string: RwLock::new(String::new()),
        });

        let pool = VaultPostgresPool {
            vault_client: vault_client.clone(),
            pool: Pool::new(manager.clone(), max_connections),
            manager,
            host: String::from(host),
            database: String::from(database),
            role: String::from(role),
        };

        pool.refresh_auth()?;

        Ok(Arc::new(pool))
    }

    fn refresh_auth(&self) -> Result<(), Error> {
        let auth = self
            .vault_client
            .read()
            .unwrap()
            .get_secret_engine_creds::<PostgresqlLogin>("database", &self.role)?;

        let data = auth.data.ok_or(Error::VaultNoDataError)?;

        let new_conn = self.get_connection_string(&data.username, &data.password);
        {
            let mut conn = self.manager.connection_string.write().unwrap();
            *conn = new_conn;
        }

        Ok(())
    }

    fn get_connection_string(&self, user: &str, password: &str) -> String {
        format!(
            "postgresql://{user}:{password}@{host}/{database}",
            user = user,
            password = password,
            host = self.host,
            database = self.database
        )
    }
}

struct Manager {
    connection_string: RwLock<String>,
}

struct WrappedConnection {
    conn_str: String,
    conn: PgConnection,
}

impl Deref for WrappedConnection {
    type Target = PgConnection;
    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl DerefMut for WrappedConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}

#[async_trait]
impl deadpool::managed::Manager<WrappedConnection, Error> for Arc<Manager> {
    async fn create(&self) -> Result<WrappedConnection, Error> {
        let this_str = { self.connection_string.read().unwrap().clone() };
        let conn = sqlx::PgConnection::connect(&this_str).await?;
        Ok(WrappedConnection {
            conn_str: this_str,
            conn,
        })
    }

    async fn recycle(
        &self,
        connection: &mut WrappedConnection,
    ) -> deadpool::managed::RecycleResult<Error> {
        let stale = { connection.conn_str != *self.connection_string.read().unwrap() };
        if stale {
            return Err(RecycleError::Message("expired user".to_string()));
        }

        connection
            .conn
            .ping()
            .await
            .map_err(|e| RecycleError::Backend(Error::from(e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
