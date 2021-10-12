use crate::error::Result;
use ergo_database::object_id::*;
use sqlx::{Connection, PgConnection};
use structopt::StructOpt;
use uuid::Uuid;

#[derive(Debug, StructOpt)]
pub struct Args {
    #[structopt(
        short,
        long,
        help = "The organization to own the API key",
        env = "ORG_ID"
    )]
    org: OrgId,
    #[structopt(short, long, help = "The user that owns the API key", env = "USER_ID")]
    user: Option<UserId>,
    #[structopt(short, long, help = "Database connection string", env = "DATABASE_URL")]
    database: String,
    #[structopt(short, long, help = "Key should not inherit user permissions")]
    no_inherit_user_permissions: bool,
    #[structopt(name = "desc", long, help = "A description for the API key")]
    description: Option<String>,
}

pub async fn make_key(
    conn: &mut PgConnection,
    org: &OrgId,
    user: Option<&UserId>,
    no_inherit_user_permissions: bool,
    description: Option<&str>,
) -> Result<String> {
    // Eventually all this code will be integrated into the ergo library itself.

    let key = ergo_auth::api_key::ApiKeyData::new();

    sqlx::query!("INSERT INTO api_keys (api_key_id, prefix, hash, org_id, user_id, inherits_user_permissions,
        description)
        VALUES
        ($1, $2, $3, $4, $5, $6, $7)",
        &key.api_key_id,
        &key.key[0..16],
        &key.hash,
        &org.0,
        user.map(|x| x.0),
        !no_inherit_user_permissions,
        description
    ).execute(&mut *conn).await?;

    println!("Key ID: {}", key.api_key_id);
    println!("Key: {}", key.key);

    Ok(key.key)
}

pub async fn main(args: Args) -> Result<()> {
    let mut conn = sqlx::PgConnection::connect(&args.database).await?;
    let mut tx = conn.begin().await?;
    make_key(
        &mut tx,
        &args.org,
        args.user.as_ref(),
        args.no_inherit_user_permissions,
        args.description.as_deref(),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
