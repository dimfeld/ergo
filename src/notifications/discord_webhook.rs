use super::{Level, Notification};
use crate::error::Result;

use serde_json::json;

fn color(level: Level) -> u32 {
    match level {
        Level::Info => 0x00ff00,
        Level::Debug => 0x329ea8, // #329ea8
        Level::Warning => 0xffff00,
        Level::Error => 0xff0000,
    }
}

pub async fn send_discord_webhook(
    client: &reqwest::Client,
    hook: &str,
    notification: &Notification,
) -> Result<()> {
    let fields = notification
        .fields()
        .into_iter()
        .map(|(name, value, inline)| {
            json!({
                "name": name,
                "value": value,
                "inline": inline,
            })
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "embeds": [
            {
                "color": color(notification.event.level()),
                "fields": fields,
            }
        ]
    });

    let url = format!("https://discord.com/api/webhooks/{}", hook);
    client.post(&url).json(&payload).send().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn sends_notification() {}
}
