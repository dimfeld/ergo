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
    use super::*;
    use ergo_database::object_id::TaskTriggerId;
    use smallvec::SmallVec;
    use sqlx::PgConnection;

    pub async fn update_triggers(
        tx: &mut PgConnection,
        task_trigger_id: &TaskTriggerId,
        periodic: &[PeriodicTaskTriggerInput],
    ) -> Result<SmallVec<[PeriodicTaskTrigger; 1]>, crate::Error> {
        let existing = sqlx::query_as!(
            PeriodicTaskTrigger,
            r##"SELECT periodic_trigger_id as "periodic_trigger_id: PeriodicTriggerId",
                name,
                schedule as "schedule: PeriodicSchedule",
                payload, enabled
            FROM periodic_triggers
            WHERE task_trigger_id=$1"##,
            task_trigger_id.0
        )
        .fetch_all(&mut *tx)
        .await?;

        let mut found_existing = SmallVec::<[&PeriodicTriggerId; 2]>::new();

        for new_value in periodic {
            if let Some(ex) = existing.iter().find(|ex| ex.schedule == new_value.schedule) {
                // Update the existing trigger

                let needs_change = false;
                if needs_change {
                    // Look up the existing task and alter its queue job.
                }

                found_existing.push(&ex.periodic_trigger_id);
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
            }
        }

        // For each item in existing that is not in found_existing, unschedule it.
        // For each item in new_value that did not match an existing value, schedule it now.
        // Also handle changes in the enabled flag.

        Ok(SmallVec::new())
    }
}
