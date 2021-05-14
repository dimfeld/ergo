use super::Notification;
use crate::error::Result;

use serde_json::json;

pub async fn send_discord_webhook(
    client: &reqwest::Client,
    hook: &str,
    notification: &Notification,
) -> Result<()> {
    // TODO Real formatting
    let payload = json!({
        "content": format!("{:?}", notification),
    });

    let url = format!("https://discord.com/api/webhooks/{}", hook);
    client.post(&url).json(&payload).send().await?;
    Ok(())
}
