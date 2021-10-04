//! Immediate mode scripts run once every time a trigger comes in. They can save a context
//! value to allow persistent state across runs.

use ergo_js::{ConsoleMessage, Runtime};
use serde::Deserialize;
use smallvec::SmallVec;

use crate::{
    scripting::{create_nonserialized_task_script_runtime, POOL},
    Error,
};

use super::{TaskJsConfig, TaskJsState};

#[derive(Debug, Deserialize)]
pub struct TaskActionInvocation {
    pub name: String,
    pub payload: serde_json::Value,
}

#[derive(Debug)]
pub struct RunTaskResult {
    pub state_changed: bool,
    pub state: TaskJsState,
    pub console: Vec<ConsoleMessage>,
    pub actions: SmallVec<[TaskActionInvocation; 4]>,
}

pub async fn run_task(
    task_name: &str,
    config: TaskJsConfig,
    mut state: TaskJsState,
) -> Result<RunTaskResult, Error> {
    let main_url = url::Url::parse(&format!("https://ergo/tasks/{}.js", task_name))
        .map_err(|e| Error::TaskScriptSetup(e.into()))?;

    let result = POOL
        .run(move || async move {
            // TODO ability to configure `allow_net`
            let mut runtime = create_nonserialized_task_script_runtime(true);

            set_up_task_env(&mut runtime, &state).map_err(|e| Error::TaskScriptSetup(e.into()))?;

            let run_result = runtime.run_main_module(main_url, config.script).await;
            let console = runtime.take_console_messages();

            match run_result {
                Ok(_) => {
                    let context_result: String = runtime
                        .get_global_value("__ergo_context")
                        .unwrap_or_else(|_| Some(String::new()))
                        .unwrap_or_else(String::new);

                    let state_changed = context_result != state.context;
                    if state_changed {
                        state.context = context_result;
                    }

                    let actions = runtime
                        .get_global_value("__ergo_actionQueue")
                        .unwrap_or_else(|_| Some(SmallVec::new()))
                        .unwrap_or_else(SmallVec::new);

                    Ok(RunTaskResult {
                        state_changed,
                        state,
                        console,
                        actions,
                    })
                }
                Err(e) => Err(Error::TaskScript { error: e, console }),
            }
        })
        .await;

    result
}

const TASK_HELPERS: &'static str = include_str!("./task_helpers.js");

fn set_up_task_env(runtime: &mut Runtime, state: &TaskJsState) -> Result<(), anyhow::Error> {
    runtime.set_global_value("__ergo_context", &state.context)?;
    runtime.execute_script("setup_task_context", TASK_HELPERS)?;
    Ok(())
}

mod tests {
    use super::*;

    #[tokio::test]
    async fn task_context() {
        let script = r##"
            let context = Ergo.getContext();
            console.dir({context});
            context.data.set('a key', 10);
            Ergo.setContext(context);
            "##;
        let config = TaskJsConfig {
            script: script.to_string(),
            timeout: None,
        };

        let state = TaskJsState {
            context: r##"{data:new Map([["a",5]])}"##.to_string(),
        };

        let result = run_task("test task", config, state).await;

        match result {
            Ok(result) => {
                eprintln!("{:?}", result.console);
                assert_eq!(
                    result.state.context,
                    r##"{data:new Map([["a",5],["a key",10]])}"##
                );
                assert_eq!(result.state_changed, true);
            }
            Err(Error::TaskScript { error, console }) => {
                eprintln!("{:?}", console);
                panic!("{:?}", error);
            }
            Err(e) => {
                panic!("{}", e);
            }
        }
    }

    #[tokio::test]
    async fn task_context_unchanged() {
        let script = r##"
            let context = Ergo.getContext();
            context.data.set('a key', 10);
            // Don't actually set the context.
            "##;
        let config = TaskJsConfig {
            script: script.to_string(),
            timeout: None,
        };

        let input_context = r##"{data:new Map([["a",5]])}"##;
        let state = TaskJsState {
            context: input_context.to_string(),
        };

        let result = run_task("test task", config, state).await;

        match result {
            Ok(result) => {
                eprintln!("{:?}", result.console);
                assert_eq!(result.state.context, input_context);
                assert_eq!(result.state_changed, false);
            }
            Err(Error::TaskScript { error, console }) => {
                eprintln!("{:?}", console);
                panic!("{:?}", error);
            }
            Err(e) => {
                panic!("{}", e);
            }
        }
    }

    #[tokio::test]
    async fn no_existing_context() {
        let script = r##"
            let context = Ergo.getContext();
            if(context === undefined) {
                Ergo.setContext('context was undefined');
            } else {
                Ergo.setContext('context was not undefined');
            }
        "##;

        let config = TaskJsConfig {
            script: script.to_string(),
            timeout: None,
        };

        let input_context = "";
        let state = TaskJsState {
            context: input_context.to_string(),
        };

        let result = run_task("test task", config, state)
            .await
            .expect("running task");
        assert_eq!(result.state_changed, true);
        assert_eq!(result.state.context, r##""context was undefined""##);
    }
}
