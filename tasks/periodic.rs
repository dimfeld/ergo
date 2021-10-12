use crate::Error;
use chrono::{DateTime, Utc};
use ergo_database::object_id::PeriodicTriggerId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub use native::*;

#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type", content = "data")]
pub enum PeriodicSchedule {
    Cron(String),
}

ergo_database::sqlx_json_decode!(PeriodicSchedule);

impl PeriodicSchedule {
    pub fn next_run(&self) -> Result<Option<DateTime<Utc>>, Error> {
        match self {
            Self::Cron(c) => {
                let schedule = cron::Schedule::from_str(c.as_str())?;
                Ok(schedule.upcoming(Utc).next())
            }
        }
    }
}

#[derive(Debug, JsonSchema, Serialize, Deserialize, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PeriodicTaskTrigger {
    pub periodic_trigger_id: PeriodicTriggerId,
    pub name: Option<String>,
    pub schedule: PeriodicSchedule,
    pub payload: serde_json::Value,
    pub enabled: bool,
}

#[derive(Debug, JsonSchema, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PeriodicTaskTriggerInput {
    name: Option<String>,
    schedule: PeriodicSchedule,
    payload: serde_json::Value,
    enabled: bool,
}

#[cfg(not(target_family = "wasm"))]
mod native {
    use crate::inputs::{enqueue_input, queue::InputQueue, EnqueueInputOptions};

    use super::*;
    use ergo_database::object_id::{InputId, OrgId, TaskId, TaskTriggerId, UserId};
    use ergo_queues::{remove_pending_job, update_pending_job, JobUpdate};
    use smallvec::SmallVec;
    use sqlx::PgConnection;

    pub async fn update_triggers(
        tx: &mut PgConnection,
        redis_key_prefix: Option<&str>,
        input_id: &InputId,
        task_trigger_id: &TaskTriggerId,
        task_trigger_local_id: &str,
        task_trigger_name: &str,
        task_id: &TaskId,
        task_name: &str,
        user_id: &UserId,
        org_id: &OrgId,
        periodic: &[PeriodicTaskTriggerInput],
    ) -> Result<SmallVec<[PeriodicTaskTrigger; 1]>, crate::Error> {
        let existing = sqlx::query!(
            r##"SELECT pt.periodic_trigger_id as "periodic_trigger_id: PeriodicTriggerId",
                schedule as "schedule: PeriodicSchedule",
                pt.payload,
                enabled,
                queue_job_id as "queue_job_id?"
            FROM periodic_triggers pt
            LEFT JOIN inputs_log il ON status='pending' AND pt.periodic_trigger_id=il.periodic_trigger_id
            WHERE pt.task_trigger_id=$1"##,
            task_trigger_id.0
        )
        .fetch_all(&mut *tx)
        .await?;

        let queue_name = InputQueue::queue_name(redis_key_prefix);
        let mut matched_existing = SmallVec::<[&PeriodicTriggerId; 2]>::new();
        let mut new_to_add = SmallVec::<[(PeriodicTriggerId, &PeriodicTaskTriggerInput); 2]>::new();

        for new_value in periodic {
            if let Some(ex) = existing.iter().find(|ex| ex.schedule == new_value.schedule) {
                // Update the existing trigger
                sqlx::query!(
                    r##"UPDATE periodic_triggers
                    SET name=$2, payload=$3, enabled=$4
                    WHERE periodic_trigger_id=$1"##,
                    ex.periodic_trigger_id.0,
                    new_value.name,
                    new_value.payload,
                    new_value.enabled
                )
                .execute(&mut *tx)
                .await?;

                if new_value.enabled && !ex.enabled {
                    // This periodic trigger has become enabled, so get ready for that.
                    new_to_add.push((ex.periodic_trigger_id.clone(), new_value));
                } else if let Some(id) = ex.queue_job_id.as_ref() {
                    if !new_value.enabled {
                        // remove the job since it's disabled now.
                        remove_pending_job(tx, queue_name.as_ref(), id).await?;
                    } else if ex.payload != new_value.payload {
                        // Update the pending job.
                        update_pending_job(
                            tx,
                            queue_name.as_ref(),
                            id,
                            &JobUpdate {
                                payload: Some(&new_value.payload),
                                run_at: None,
                            },
                        )
                        .await?;
                    }
                }

                matched_existing.push(&ex.periodic_trigger_id);
            } else {
                let pt_id = PeriodicTriggerId::new();
                sqlx::query!(
                   "INSERT INTO periodic_triggers (periodic_trigger_id, task_trigger_id, name, schedule, payload, enabled)
                   VALUES
                   ($1, $2, $3, $4, $5, $6)",
                    pt_id.0,
                    task_trigger_id.0,
                    new_value.name,
                    sqlx::types::Json(&new_value.schedule) as _,
                    new_value.payload,
                    new_value.enabled
                ).execute(&mut *tx).await?;

                if new_value.enabled {
                    new_to_add.push((pt_id, new_value));
                }
            }
        }

        // For each item in existing that is not in matched_existing, unschedule it and delete it
        // from the database.
        for existing in existing
            .iter()
            .filter(|ex| matched_existing.contains(&&ex.periodic_trigger_id) == false)
        {
            if let Some(job_id) = existing.queue_job_id.as_deref() {
                remove_pending_job(tx, queue_name.as_ref(), job_id).await?;
            }

            sqlx::query!(
                "DELETE FROM periodic_triggers WHERE periodic_trigger_id=$1",
                existing.periodic_trigger_id.0
            )
            .execute(&mut *tx)
            .await?;
        }

        if !new_to_add.is_empty() {
            let info = sqlx::query!(
                r##"
                SELECT payload_schema
                FROM inputs
                WHERE input_id=$1
                "##,
                input_id.0
            )
            .fetch_one(&mut *tx)
            .await?;

            for (periodic_trigger_id, trigger) in new_to_add {
                if let Some(next_date) = trigger.schedule.next_run()? {
                    enqueue_input(EnqueueInputOptions {
                        pg: tx,
                        notifications: None,
                        org_id: org_id.clone(),
                        user_id: user_id.clone(),
                        task_id: task_id.clone(),
                        task_name: task_name.to_string(),
                        input_id: input_id.clone(),
                        task_trigger_id: task_trigger_id.clone(),
                        task_trigger_local_id: task_trigger_local_id.to_string(),
                        task_trigger_name: task_trigger_name.to_string(),
                        periodic_trigger_id: Some(periodic_trigger_id),
                        payload_schema: &info.payload_schema,
                        payload: trigger.payload.clone(),
                        redis_key_prefix: redis_key_prefix.as_deref(),
                        trigger_at: Some(next_date),
                    })
                    .await?;
                }
            }
        }

        Ok(SmallVec::new())
    }
}
