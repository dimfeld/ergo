use crate::{
    object_id::{ActionCategoryId, OrgId, UserId},
    DatabaseConfiguration,
};
use anyhow::{anyhow, Result};
use futures::Future;
use lazy_static::lazy_static;
use sqlx::{postgres::PgConnectOptions, ConnectOptions, Executor, PgConnection};
use std::str::FromStr;

#[derive(Clone)]
pub struct TestDatabase {
    pub config: DatabaseConfiguration,
    pub name: String,
    pub pool: sqlx::postgres::PgPool,
    global_connect_options: PgConnectOptions,
}

impl TestDatabase {
    pub async fn drop_db(&self) -> Result<()> {
        let mut conn = self.global_connect_options.connect().await?;
        sqlx::query(&format!(r##"DROP DATABASE "{}" (FORCE)"##, self.name))
            .execute(&mut conn)
            .await?;
        Ok(())
    }
}

pub struct DatabaseUser {
    pub org_id: OrgId,
    pub user_id: UserId,
    pub action_category_id: ActionCategoryId,
    pub password: Option<String>,
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

pub async fn run_database_test<F, R>(f: F) -> ()
where
    F: FnOnce(TestDatabase) -> R,
    R: Future<Output = Result<(), anyhow::Error>>,
{
    let (database, _, _) = create_database().await.expect("Creating database");
    f(database.clone()).await.unwrap();
    database.drop_db().await.expect("Cleaning up");
}

pub async fn create_database() -> Result<(TestDatabase, OrgId, DatabaseUser)> {
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
        database: format!("ergo_test_{}", crate::new_uuid().to_simple()),
        host,
        port,
    };

    println!("Database name: {}", config.database);

    let global_connect_options = PgConnectOptions::new()
        .port(port)
        .host(&config.host)
        .username(&user)
        .password(&password);

    let mut global_conn = global_connect_options.connect().await?;

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
  CREATE ROLE ergo_web INHERIT IN ROLE ergo_user {web_password};
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;

DO $$BEGIN
  CREATE ROLE ergo_backend INHERIT IN ROLE ergo_user {backend_password};
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;

DO $$BEGIN
  CREATE ROLE ergo_enqueuer INHERIT IN ROLE ergo_user {enqueuer_password};
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;
            "##,
        web_password = password_sql("WEB"),
        backend_password = password_sql("BACKEND"),
        enqueuer_password = password_sql("ENQUEUER"),
    );

    global_conn.execute(roles_query.as_str()).await?;
    drop(global_conn);

    let pool_options = PgConnectOptions::new()
        .port(port)
        .host(&config.host)
        .database(&config.database)
        .username(&user)
        .password(&password);
    let pool = sqlx::PgPool::connect_with(pool_options).await?;

    sqlx::migrate!("../migrations").run(&pool).await?;

    let mut conn = pool.acquire().await?;
    let admin_user = populate_database(&mut conn).await?;
    drop(conn);

    Ok((
        TestDatabase {
            pool,
            name: config.database.clone(),
            global_connect_options,
            config,
        },
        admin_user.org_id.clone(),
        admin_user,
    ))
}

pub const PASSWORD: &'static str = "test password";
const PASSWORD_HASH: &'static str = "$argon2id$v=19$m=15360,t=2,p=1$PUpyHXvHTSOKvr9Sc6vK8g$GSyd7TMMKrS7bkObHL3+aOtRmULRJTNP1xLP4C/3zzY";

lazy_static! {
    static ref ADMIN_USER_ID: UserId =
        UserId::from_str(std::env::var("ADMIN_USER_ID").unwrap().as_str()).unwrap();
}

async fn populate_database(conn: &mut PgConnection) -> Result<DatabaseUser, anyhow::Error> {
    let user_id = ADMIN_USER_ID.clone();
    let org_id = OrgId::new();
    let action_category_id = ActionCategoryId::new();

    let query = format!(
        r##"
        INSERT INTO orgs (org_id, name) VALUES
          ('{org_id}', 'Test Org');

        INSERT INTO users (user_id, active_org_id, name, email, password_hash) VALUES
          ('{user_id}', '{org_id}', 'Test Admin User', 'user@example.com', '{password_hash}');

        -- Temporary until API supporst creating action categories.
        INSERT INTO action_categories(action_category_id, name) VALUES
            ('{action_category_id}', 'General');
        "##,
        user_id = &user_id.0,
        org_id = &org_id.0,
        action_category_id = &action_category_id.0,
        password_hash = escape(PASSWORD_HASH)
    );

    conn.execute(query.as_str()).await?;

    Ok(DatabaseUser {
        user_id,
        org_id,
        action_category_id,
        password: Some(PASSWORD.to_string()),
    })
}
