use super::{Error, Level, Notification};

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
) -> Result<(), Error> {
    let desc = notification.event.description();
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
        "content": desc,
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
    use ergo_database::object_id::TaskId;
    use uuid::Uuid;

    use crate::NotifyEvent;

    #[tokio::test]
    async fn sends_notification() {
        dotenv::dotenv().ok();

        let hook = std::env::var("TEST_DISCORD_WEBHOOK_URL").unwrap_or_else(|_| String::new());
        if hook.is_empty() {
            return;
        }

        let notification = super::Notification {
            event: NotifyEvent::ActionSuccess,
            task_id: TaskId::new(),
            task_name: "a test task".to_string(),
            local_id: "the local id".to_string(),
            local_object_name: "the local object name".to_string(),
            local_object_id: Some(Uuid::new_v4()),
            payload: Some(serde_json::json!({ "payload_value": 5})),
            error: None,
            log_id: Some(uuid::Uuid::new_v4()),
        };

        super::send_discord_webhook(&reqwest::Client::new(), hook.as_str(), &notification)
            .await
            .expect("Sending notification");
    }
}
