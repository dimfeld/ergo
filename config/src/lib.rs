#[derive(Clone, Debug)]
pub struct Config {
    pub vault_client: vault::AppRoleVaultClient,

    pub database: Option<String>,
    pub database_host: String,
    pub database_role: Option<String>,

    pub shutdown: graceful_shutdown::GracefulShutdownConsumer,
}
