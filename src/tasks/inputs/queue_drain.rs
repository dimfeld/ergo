use crate::{
    error::Error,
    queues::{
        postgres_drain::{Drainer, QueueStageDrain, QueueStageDrainConfig},
        Job, JobId,
    },
};

use async_trait::async_trait;
use sqlx::{query_as, Postgres, Transaction};

struct QueueDrainer {}

#[async_trait]
impl Drainer for QueueDrainer {
    async fn get(&self, tx: &mut Transaction<Postgres>) -> Result<Vec<Job>, Error> {
        let results = sqlx::query!(
            r##"SELECT event_queue_id, task_id, task_trigger_id, input_id, payload
            FROM event_queue ORDER BY event_queue_id LIMIT 50"##
        )
        .fetch_all(&mut *tx)
        .await?;

        if let Some(max_id) = results.last().map(|r| r.event_queue_id) {
            sqlx::query!("DELETE FROM event_queue WHERE event_queue_id < $1", max_id)
                .execute(&mut *tx)
                .await?;
        }

        results
            .into_iter()
            .map(|row| {
                let payload = super::InputInvocation {
                    task_id: row.task_id,
                    task_trigger_id: row.task_trigger_id,
                    input_id: row.input_id,
                    payload: row.payload.unwrap_or(serde_json::Value::Null),
                };

                Job::from_json_payload(JobId::Value(&row.event_queue_id.to_string()), &payload)
            })
            .collect::<Result<Vec<Job>, serde_json::Error>>()
            .map_err(Error::from)
    }
}
