use crate::{database::VaultPostgresPoolAuth, graceful_shutdown, vault::VaultClientTokenData};

#[derive(Clone, Debug)]
pub struct Config<T: VaultClientTokenData> {
    pub database: Option<String>,
    pub database_host: String,
    pub database_auth: VaultPostgresPoolAuth<T>,

    pub redis_host: String,

    pub shutdown: graceful_shutdown::GracefulShutdownConsumer,
}
