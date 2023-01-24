#![allow(clippy::bool_assert_comparison)]

pub mod actions;
pub mod dataflow;
mod error;
pub mod inputs;
pub mod periodic;
#[cfg(not(target_family = "wasm"))]
pub mod queue_drain_runner;
pub mod scripting;
pub mod state_machine;

use actions::{Action, TaskAction};
use ergo_database::object_id::{InputId, PeriodicTriggerId, TaskId, TaskTriggerId};
pub use error::*;
use inputs::Input;
#[cfg(not(target_family = "wasm"))]
pub use native::*;
pub use periodic::{PeriodicSchedule, PeriodicTaskTrigger, PeriodicTaskTriggerInput};

use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum TaskConfig {
    StateMachine(state_machine::StateMachineConfig),
    Js(scripting::TaskJsConfig),
    DataFlow(dataflow::DataFlowConfig),
}

impl TaskConfig {
    pub fn validate(
        &self,
        actions: &FxHashMap<String, Action>,
        inputs: &FxHashMap<String, Input>,
        task_triggers: &FxHashMap<String, TaskTrigger>,
        task_actions: &FxHashMap<String, TaskAction>,
    ) -> Result<(), TaskValidateErrors> {
        let errors = match self {
            Self::StateMachine(machines) => {
                let mut errors = Vec::new();
                for m in machines {
                    errors.extend_from_slice(
                        m.validate(actions, inputs, task_triggers, task_actions)
                            .as_slice(),
                    );
                }
                errors
            }
            // TODO Some sort of validation for scripts.
            // At least do a syntax check.
            Self::Js(_) => Vec::new(),
            // TODO
            Self::DataFlow(_) => Vec::new(),
        };

        if errors.is_empty() {
            Ok(())
        } else {
            Err(TaskValidateErrors(errors))
        }
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn default_state(&self) -> TaskState {
        match self {
            Self::StateMachine(config) => {
                TaskState::StateMachine(config.iter().map(|s| s.default_state()).collect())
            }
            Self::Js(config) => TaskState::Js(config.default_state()),
            Self::DataFlow(config) => TaskState::DataFlow(config.default_state()),
        }
    }
}

#[derive(Debug, JsonSchema, Serialize, Deserialize)]
pub struct TaskTrigger {
    pub task_trigger_id: TaskTriggerId,
    pub task_id: TaskId,
    pub input_id: InputId,
    pub name: String,
    pub description: Option<String>,
    #[schemars(with = "Option<String>")]
    pub last_payload: Option<Box<serde_json::value::RawValue>>,
    pub periodic: Option<Vec<PeriodicTaskTrigger>>,
}

#[cfg(not(target_family = "wasm"))]
mod native {
    use super::*;
    use crate::{
        actions::{
            enqueue_actions,
            execute::{
                validate_and_prepare_invocation, ExecuteError, PrepareInvocationAction,
                ScriptOrTemplate,
            },
            template::TemplateFields,
            ActionInvocation, ActionInvocations, ActionStatus, TaskActionTemplate,
        },
        dataflow::DataFlowState,
        inputs::{enqueue_input, EnqueueInputOptions, InputInvocation, InputStatus},
        scripting::TaskJsState,
        state_machine::{StateMachineStates, StateMachineWithData},
        TaskConfig,
    };
    use chrono::{DateTime, Utc};
    use ergo_database::{
        new_uuid,
        object_id::{
            AccountId, ActionId, InputId, OrgId, TaskId, TaskTemplateId, TaskTriggerId, UserId,
        },
        sql_insert_parameters,
        transaction::serializable,
        PostgresPool,
    };
    use ergo_notifications::{Notification, NotificationManager, NotifyEvent};
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use smallvec::SmallVec;
    use sqlx::{types::Json, FromRow};
    use tracing::{event, instrument, Level};
    use uuid::Uuid;

    #[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
    #[serde(tag = "type", content = "data")]
    pub enum TaskState {
        StateMachine(StateMachineStates),
        Js(TaskJsState),
        DataFlow(DataFlowState),
    }

    #[derive(Serialize, Deserialize, FromRow)]
    pub struct Task {
        pub task_id: TaskId,
        pub org_id: Uuid,
        pub name: String,
        pub description: Option<String>,
        pub enabled: bool,
        pub task_template_id: uuid::Uuid,
        pub task_template_version: TaskTemplateId,
        pub config: Json<TaskConfig>,
        pub state: Json<TaskState>,
        pub created: DateTime<Utc>,
        pub modified: DateTime<Utc>,
    }

    impl Task {
        /// Apply an input to a task.
        /// Instead of acting on an existing task instance, this loads the task
        /// and applies the input inside a serializable transaction, to ensure that
        /// the applied input doesn't have a race condition with any other concurrent
        /// inputs to the same task.
        #[instrument(skip(pool, notifications))]
        pub async fn apply_input(
            pool: &PostgresPool,
            notifications: Option<NotificationManager>,
            redis_key_prefix: Option<String>,
            reschedule_periodic_task_on_error: bool,
            invocation: InputInvocation,
        ) -> Result<(), Error> {
            let mut conn = pool.acquire().await?;

            let inv = invocation.clone();
            let not = notifications.clone();
            let rkp = redis_key_prefix.clone();

            let result = serializable(&mut conn, 5, move |tx| {
                let InputInvocation{
                    payload,
                    inputs_log_id: input_arrival_id,
                    task_id,
                    task_trigger_id,
                    user_id,
                    periodic_trigger_id,
                    ..
                } = inv.clone();
                let notifications = not.clone();
                let redis_key_prefix = rkp.clone();

                Box::pin(async move {
                    #[derive(Debug, Deserialize)]
                    struct TaskAction {
                        task_action_local_id: String,
                        task_action_name: String,
                        task_action_template: Option<TaskActionTemplate>,
                        action_id: ActionId,
                        action_template_fields: TemplateFields,
                        action_executor_template: ScriptOrTemplate,
                        executor_id: String,
                        account_id: Option<AccountId>,
                        account_required: bool,
                        account_fields: Option<TaskActionTemplate>,
                        account_expires: Option<DateTime<Utc>>,
                    }

                    #[derive(Debug, FromRow)]
                    struct TaskInputData {
                        task_trigger_local_id: String,
                        config: Json<TaskConfig>,
                        state: Json<TaskState>,
                        org_id: OrgId,
                        task_name: String,
                        task_trigger_name: String,
                        task_actions: Json<SmallVec<[TaskAction; 4]>>,
                        periodic_trigger_id: Option<PeriodicTriggerId>,
                    }

                    let task = sqlx::query_as!(TaskInputData,
                            r##"
                            SELECT
                            task_trigger_local_id as "task_trigger_local_id!",
                            compiled as "config!: Json<TaskConfig>",
                            state as "state!: Json<TaskState>",
                            tasks.org_id as "org_id: OrgId",
                            tasks.name as task_name,
                            tt.name as task_trigger_name,
                            pt.periodic_trigger_id as "periodic_trigger_id: Option<PeriodicTriggerId>",
                            jsonb_agg(jsonb_build_object(
                                'task_action_local_id', ta.task_action_local_id,
                                'task_action_name', ta.name,
                                'task_action_template', NULLIF(ta.action_template, 'null'::jsonb),
                                'action_id', ta.action_id,
                                'account_id', ta.account_id,
                                'account_fields', accounts.fields,
                                'account_expires', accounts.expires,
                                'action_template', ta.action_template,
                                'action_template_fields', ac.template_fields,
                                'account_required', ac.account_required,
                                'executor_id', ac.executor_id,
                                'action_executor_template', ac.executor_template
                            )) as "task_actions!: _"
                            FROM tasks
                            JOIN task_templates USING (task_template_id, task_template_version)
                            JOIN task_triggers tt ON tt.task_id=$1 AND task_trigger_id=$2
                            JOIN task_actions ta ON ta.task_id=$1
                            LEFT JOIN periodic_triggers pt on pt.task_trigger_id=tt.task_trigger_id AND pt.periodic_trigger_id=$3 AND pt.enabled
                            JOIN actions ac USING(action_id)
                            LEFT JOIN accounts USING(account_id)
                            WHERE tasks.task_id=$1
                            GROUP BY task_trigger_local_id, compiled, state, tasks.org_id, task_name,
                                task_trigger_name, periodic_trigger_id"##,
                            task_id.0,
                            task_trigger_id.0,
                            periodic_trigger_id as _
                        )
                        .fetch_optional(&mut *tx)
                        .await?;

                    let task = task.ok_or(Error::NotFound)?;

                    let TaskInputData {
                        task_trigger_local_id, config, state, org_id, task_name, task_trigger_name, task_actions, periodic_trigger_id: found_periodic_trigger
                    } = task;

                    if periodic_trigger_id.is_some() && found_periodic_trigger.is_none() {
                        // If this run is for a periodic trigger that doesn't exist anymore or was
                        // disabled, then don't do anything.
                        return Err(Error::PeriodicTaskDeleted);
                    }

                    let (new_data, log_info, actions, changed) = match (config.0, state.0) {
                        (TaskConfig::StateMachine(machine), TaskState::StateMachine(state)) => {
                            let num_machines = machine.len();
                            let mut new_data = StateMachineStates::with_capacity(num_machines);
                            let mut actions = ActionInvocations::new();
                            let mut changed = false;
                            for (idx, (machine, state)) in machine
                                .into_iter()
                                .zip(state.into_iter())
                                .enumerate() {
                                    let mut m = StateMachineWithData::new(task_id, idx, machine, state);
                                    let this_actions = m
                                      .apply_trigger(
                                          &task_trigger_local_id,
                                          &user_id,
                                          &Some(input_arrival_id),
                                          Some(&payload),
                                      ).await
                                      .map_err(Error::from)?;

                                  let (data, this_changed) = m.take();
                                  new_data.push(data);
                                  actions.extend(this_actions.into_iter());
                                  changed = changed || this_changed;
                            }

                            (TaskState::StateMachine(new_data), serde_json::Value::Null, actions, changed)
                        },
                        (TaskConfig::StateMachine(_), _) =>  {
                            return Err(Error::ConfigStateMismatch("StateMachine"))
                        },
                        (TaskConfig::Js(config), TaskState::Js(state)) => {
                            let run_result = scripting::immediate::run_task(&task_name, config, state, payload.clone()).await?;
                            let actions = run_result.actions.into_iter().map(|action| {
                                ActionInvocation{
                                    task_id,
                                    payload: action.payload,
                                    input_arrival_id: Some(input_arrival_id),
                                    user_id,
                                    task_action_local_id: action.name,
                                    actions_log_id: new_uuid(),
                                }
                            }).collect::<ActionInvocations>();

                            // TODO Return console messages here
                            (TaskState::Js(run_result.state), serde_json::Value::Null ,actions, run_result.state_changed)
                        },
                        (TaskConfig::Js(_), _) =>  {
                            return Err(Error::ConfigStateMismatch("Js"))
                        },
                        (TaskConfig::DataFlow(config), TaskState::DataFlow(state)) => {
                            let (state, log, actions) = config.evaluate_trigger(&task_name, state, task_trigger_id, &task_trigger_local_id, payload.clone()).await?;
                            let actions = actions.into_iter().map(|action| {
                                ActionInvocation{
                                    task_id,
                                    payload: action.payload,
                                    input_arrival_id: Some(input_arrival_id),
                                    user_id,
                                    task_action_local_id: action.name,
                                    actions_log_id: new_uuid(),
                                }
                            }).collect::<ActionInvocations>();

                            let log_out = serde_json::to_value(log)?;

                            (TaskState::DataFlow(state), log_out, actions, true)
                        }
                        (TaskConfig::DataFlow(_), _) => {
                            return Err(Error::ConfigStateMismatch("DataFlow"))
                        }
                    };

                    if changed {
                        event!(Level::INFO, state=?new_data, "New state");
                        sqlx::query!(
                            r##"UPDATE tasks
                            SET state = $1::jsonb
                            WHERE task_id = $2;
                            "##,
                            serde_json::value::to_value(&new_data)?,
                            *task_id,
                        )
                        .execute(&mut *tx)
                        .await?;
                    }

                    if !actions.is_empty() {
                        event!(Level::INFO, ?actions, "Enqueueing actions");
                        event!(Level::DEBUG, ?task_actions);
                        let q = format!(
                            "INSERT INTO actions_log (task_id, task_action_local_id, actions_log_id, inputs_log_id, payload, status)
                            VALUES
                            {}
                            ",
                            sql_insert_parameters::<6>(actions.len())
                        );

                        let mut log_query = sqlx::query(&q);

                        for action in &actions {
                            let task_action = task_actions.iter().find(|a| a.task_action_local_id == action.task_action_local_id)
                                .ok_or_else(|| Error::TaskActionNotFound(action.task_action_local_id.clone()))?;

                            // Prepare the invocation. We won't actu
                            let invocation_action = PrepareInvocationAction{
                                action_id: &task_action.action_id,
                                executor_id: task_action.executor_id.as_str(),
                                account_required: task_action.account_required,
                                account_id: &task_action.account_id,
                                account_expires: task_action.account_expires,
                                account_fields: task_action.account_fields.clone(),
                                action_template_fields: &task_action.action_template_fields,
                                task_action_template: task_action.task_action_template.clone(),
                                action_executor_template: &task_action.action_executor_template
                            };

                            let executor = actions::execute::EXECUTOR_REGISTRY
                                .get(task_action.executor_id.as_str())
                                .ok_or_else(|| ActionValidateErrors::from(ActionValidateError::UnknownExecutor(task_action.executor_id.clone())))?;

                            validate_and_prepare_invocation(executor, &action.payload, invocation_action).await
                                .map_err(|e| ExecuteError{
                                    task_id,
                                    task_action_local_id: action.task_action_local_id.clone(),
                                    task_action_name: task_action.task_action_name.clone(),
                                    error: e,
                                })?;

                            log_query = log_query
                                .bind(action.task_id)
                                .bind(&action.task_action_local_id)
                                .bind(action.actions_log_id)
                                .bind(action.input_arrival_id)
                                .bind(&action.payload)
                                .bind(ActionStatus::Pending);
                        }

                        log_query.fetch_all(&mut *tx).await?;
                        enqueue_actions(&mut *tx, &actions, &redis_key_prefix).await?;
                    }

                    if let Some(notifications) = notifications {
                        let input_notification = Notification{
                            event: NotifyEvent::InputProcessed,
                            payload: Some(payload),
                            task_id,
                            task_name,
                            local_id: task_trigger_local_id,
                            local_object_name: task_trigger_name,
                            local_object_id: Some(task_trigger_id.into()),
                            error: None,
                            log_id: Some(input_arrival_id),
                        };
                        notifications.notify(tx, &org_id, input_notification).await?;
                    }

                    Ok::<serde_json::Value, Error>(log_info)
                })
            })
            .await;

            let (log_info, status, retval) = match result {
                Ok(log_info) => (log_info, InputStatus::Success, Ok(())),
                Err(Error::PeriodicTaskDeleted) => {
                    // This isn't an error, it just means that the task started to run when it
                    // shouldn't have. Just remove the log entry and pretend it didn't run.
                    event!(Level::INFO, "Periodic task no longer exists");
                    sqlx::query!(
                        "DELETE FROM inputs_log WHERE inputs_log_id=$1",
                        invocation.inputs_log_id
                    )
                    .execute(pool)
                    .await?;

                    return Ok(());
                }
                Err(e) => {
                    event!(Level::ERROR, err=?e, "Error applying input");
                    (
                        serde_json::json!({ "msg": e.to_string(), "info": format!("{:?}", e) }),
                        InputStatus::Error,
                        Err(e),
                    )
                }
            };

            event!(Level::INFO, input_arrival_id=%invocation.inputs_log_id, ?status, ?log_info, "Updating input status");
            sqlx::query!(
                "UPDATE inputs_log SET status=$2, info=$3, updated=now() WHERE inputs_log_id=$1",
                invocation.inputs_log_id,
                status as _,
                log_info
            )
            .execute(pool)
            .await?;

            // If this was a periodic trigger, enqueue it again.
            if let Some(periodic_id) = invocation
                .periodic_trigger_id
                .filter(|_| retval.is_ok() || reschedule_periodic_task_on_error)
            {
                let info = sqlx::query!(
                    r##"SELECT
                    pt.payload,
                    pt.schedule AS "schedule: PeriodicSchedule",
                    pt.enabled AS pt_enabled,
                    pt.run_as_user AS "run_as_user: UserId",
                    tasks.enabled AS task_enabled,
                    task_trigger_id AS "task_trigger_id: TaskTriggerId",
                    task_trigger_local_id,
                    tasks.name AS task_name,
                    tt.name AS task_trigger_name,
                    input_id AS "input_id: InputId",
                    inputs.payload_schema,
                    task_id as "task_id: TaskId",
                    org_id as "org_id: OrgId"
                    FROM periodic_triggers pt
                    JOIN task_triggers tt USING (task_trigger_id)
                    JOIN inputs USING (input_id)
                    JOIN tasks USING (task_id)
                    WHERE pt.periodic_trigger_id=$1
                    "##,
                    periodic_id.0
                )
                .fetch_optional(pool)
                .await?;

                if let Some(info) = info {
                    if let Some(next_time) = info
                        .schedule
                        .next_run()?
                        .filter(|_| info.pt_enabled && info.task_enabled)
                    {
                        let mut conn = pool.acquire().await?;
                        enqueue_input(EnqueueInputOptions {
                            pg: &mut conn,
                            notifications,
                            org_id: info.org_id,
                            user_id: info.run_as_user,
                            task_id: info.task_id,
                            task_name: info.task_name,
                            input_id: info.input_id,
                            task_trigger_id: info.task_trigger_id,
                            task_trigger_local_id: info.task_trigger_local_id,
                            task_trigger_name: info.task_trigger_name,
                            periodic_trigger_id: Some(periodic_id),
                            payload_schema: &info.payload_schema,
                            payload: info.payload,
                            redis_key_prefix: redis_key_prefix.as_deref(),
                            trigger_at: Some(next_time),
                        })
                        .await?;
                    }
                }
            }

            retval
        }
    }
}
