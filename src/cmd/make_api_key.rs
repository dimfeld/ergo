use crate::error::Result;
use sqlx::Connection;
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
    org: Uuid,
    #[structopt(short, long, help = "The user that owns the API key", env = "USER_ID")]
    user: Option<Uuid>,
    #[structopt(short, long, help = "Database connection string", env = "DATABASE_URL")]
    database: String,
    #[structopt(short, long, help = "Key should not inherit user permissions")]
    no_inherit_user_permissions: bool,
    #[structopt(name = "desc", long, help = "A description for the API key")]
    description: Option<String>,
}

pub async fn main(args: Args) -> Result<()> {
    let mut conn = sqlx::PgConnection::connect(&args.database).await?;
    let mut tx = conn.begin().await?;

    // Eventually all this code will be integrated into the ergo library itself.

    let key = crate::auth::api_key::ApiKeyData::new();

    sqlx::query!(
        "INSERT INTO user_entity_ids (user_entity_id) VALUES ($1)",
        &key.api_key_id
    )
    .execute(&mut tx)
    .await?;

    sqlx::query!("INSERT INTO api_keys (api_key_id, prefix, hash, org_id, user_id, inherits_user_permissions,
        description)
        VALUES
        ($1, $2, $3, $4, $5, $6, $7)",
        &key.api_key_id,
        &key.key[0..16],
        &key.hash,
        &args.org,
        args.user.as_ref(),
        !args.no_inherit_user_permissions,
        args.description.as_ref()
    ).execute(&mut tx).await?;

    tx.commit().await?;

    println!("Key ID: {}", key.api_key_id);
    println!("Key: {}", key.key);

    Ok(())
}
