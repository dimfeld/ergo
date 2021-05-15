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
    // TODO Real formatting
    let payload = json!({
        "embeds": [
            {
                "color": color(notification.event.level()),
                "fields": [
                    {
                        "name": "Task",
                        "value": &notification.task_name,
                        "inline": true
                    },
                    {
                        "name": notification.event.local_object_type(),
                        "value": &notification.local_object_name,
                        "inline": true
                    },
                    {
                        "name": "Message",
                        "value": format!("{:?}", notification),
                    },
                    {
                        "name": "Log ID",
                        "value": notification.log_id.map(|u| u.to_string()),
                    }
                ]
            }
        ]
    });

    let url = format!("https://discord.com/api/webhooks/{}", hook);
    client.post(&url).json(&payload).send().await?;
    Ok(())
}
