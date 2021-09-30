//! Immediate mode scripts run once every time a trigger comes in. They can save a context
//! value to allow persistent state across runs.

use crate::actions::ActionInvocations;

use super::{
    runtime::{create_executor_runtime, POOL},
    TaskJsConfig, TaskJsState,
};

#[derive(Debug)]
pub struct RunTaskResult {
    pub state_changed: bool,
    pub state: TaskJsState,
    pub actions: ActionInvocations,
}

pub async fn run_task(
    config: TaskJsConfig,
    state: TaskJsState,
) -> Result<RunTaskResult, anyhow::Error> {
    let result = POOL
        .run(move || async move {
            let mut runtime = create_executor_runtime();
            let main_url = url::Url::parse("https://ergo/main.js").unwrap();
            let main_mod_id = runtime
                .load_main_module(&main_url, Some(config.script))
                .await?;
            let mod_done = runtime.mod_evaluate(main_mod_id);
            //
            // TODO timeout
            runtime.run_event_loop(false).await?;
            let x = mod_done.await?;

            Ok::<_, anyhow::Error>(RunTaskResult {
                state_changed: false,
                state,
                actions: ActionInvocations::new(),
            })
        })
        .await;

    result
}
