use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use sqlx::{postgres::PgConnectOptions, ConnectOptions, Executor, PgConnection};
use uuid::Uuid;

use ergo::{cmd::make_api_key, service_config::DatabaseConfiguration};

pub struct TestDatabase {
    pub config: DatabaseConfiguration,
    pub name: String,
    /// The ID of the precreated organization.
    pub org_id: Uuid,
    /// The ID of the precreated admin user.
    pub user_id: Uuid,
    /// The password for the precreated user.
    pub user_password: &'static str,
    /// An API key for the precreated user.
    pub user_api_key: String,
}

fn escape(s: &str) -> String {
    s.replace('\\', r##"\\"##).replace('\'', r##"\'"##)
}

fn password_sql(role: &str) -> String {
    if let Ok(pwd) = std::env::var(&format!("DATABASE_ROLE_{}_PASSWORD", role)) {
        format!("LOGIN PASSWORD '{}'", escape(&pwd))
    } else {
        String::new()
    }
}

pub async fn create_database() -> Result<TestDatabase> {
    dotenv::dotenv().ok();
    let host = std::env::var("TEST_DATABASE_HOST")
        .or_else(|_| std::env::var("DATABASE_HOST"))
        .unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("TEST_DATABASE_PORT")
        .or_else(|_| std::env::var("DATABASE_PORT"))
        .map_err(|e| anyhow!(e))
        .and_then(|val| val.parse::<u16>().map_err(|e| anyhow!(e)))
        .unwrap_or(5432);
    let user = std::env::var("TEST_DATABASE_USER").unwrap_or_else(|_| "postgres".to_string());
    let password = std::env::var("TEST_DATABASE_PASSWORD").unwrap_or_else(|_| "".to_string());

    let config = DatabaseConfiguration {
        database: format!("ergo-test-{}", Uuid::new_v4()),
        host,
        port,
    };

    let mut global_conn = PgConnectOptions::new()
        .port(port)
        .host(&config.host)
        .username(&user)
        .password(&password)
        .connect()
        .await?;

    sqlx::query(&format!(r##"CREATE DATABASE "{}""##, config.database))
        .execute(&mut global_conn)
        .await?;

    // The roles are global, but need to be set up. The migrations normally handle this but for
    // tests we need to make sure that the passwords are set.
    let roles_query = format!(
        r##"
DO $$BEGIN
  CREATE ROLE ergo_user INHERIT;
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;

DO $$BEGIN
  CREATE ROLE ergo_web NOINHERIT IN ROLE ergo_user {web_password};
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;

DO $$BEGIN
  CREATE ROLE ergo_backend NOINHERIT IN ROLE ergo_user {backend_password};
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;

DO $$BEGIN
  CREATE ROLE ergo_enqueuer NOINHERIT IN ROLE ergo_user {enqueuer_password};
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;
            "##,
        web_password = password_sql("WEB"),
        backend_password = password_sql("BACKEND"),
        enqueuer_password = password_sql("ENQUEUER"),
    );

    global_conn.execute(roles_query.as_str()).await?;
    drop(global_conn);

    let mut database_conn = PgConnectOptions::new()
        .port(port)
        .host(&config.host)
        .database(&config.database)
        .username(&user)
        .password(&password)
        .connect()
        .await?;
    sqlx::migrate!("./migrations")
        .run(&mut database_conn)
        .await?;

    let (user_id, org_id, user_api_key) = populate_database(&mut database_conn).await?;

    Ok(TestDatabase {
        name: config.database.clone(),
        config,
        user_id,
        user_password: PASSWORD,
        user_api_key,
        org_id,
    })
}

pub const PASSWORD: &'static str = "test password";
const PASSWORD_HASH: &'static str = "$argon2id$v=19$m=15360,t=2,p=1$PUpyHXvHTSOKvr9Sc6vK8g$GSyd7TMMKrS7bkObHL3+aOtRmULRJTNP1xLP4C/3zzY";

lazy_static! {
    static ref ADMIN_USER_ID: Uuid =
        Uuid::parse_str(std::env::var("ADMIN_USER_ID").unwrap().as_str()).unwrap();
}

async fn populate_database(conn: &mut PgConnection) -> Result<(Uuid, Uuid, String)> {
    let user_id = ADMIN_USER_ID.clone();
    let org_id = Uuid::new_v4();

    let query = format!(
        r##"
        INSERT INTO user_entity_ids (user_entity_id) VALUES
          ('{org_id}'),
          ('{user_id}')
          ON CONFLICT DO NOTHING;

        INSERT INTO orgs (org_id, name) VALUES
          ('{org_id}', 'Test Org');

        INSERT INTO users (user_id, active_org_id, name, email, password_hash) VALUES
          ('{user_id}', '{org_id}', 'Test Admin User', 'user@example.com', '{password_hash}');
        "##,
        user_id = user_id,
        org_id = org_id,
        password_hash = escape(PASSWORD_HASH)
    );

    conn.execute(query.as_str()).await?;

    let key = make_api_key::make_key(conn, &org_id, Some(&user_id), false, None).await?;

    Ok((user_id, org_id, key))
}
