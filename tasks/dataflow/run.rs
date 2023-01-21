use ergo_js::{worker::JsWorker, ConsoleMessage};
use futures::FutureExt;
use fxhash::FxHashMap;
use serde::de::DeserializeOwned;

use super::{DataFlowConfig, DataFlowState};
use crate::{Error, Result};

pub const DATAFLOW_ENV_CODE: &str = include_str!("../js_helpers/dist/dataflow.js");

pub struct DataFlowRunner {
    worker: ergo_js::worker::JsWorker,
}

impl DataFlowRunner {
    pub async fn new(
        config: &DataFlowConfig,
        state: &DataFlowState,
    ) -> Result<DataFlowRunner, Error> {
        let state: FxHashMap<&str, &str> = config
            .nodes
            .iter()
            .zip(state.nodes.iter())
            .map(|(c, s)| (c.name.as_str(), s.as_str()))
            .collect();

        let state = serde_json::to_string(&state)?;
        let code = format!("globalThis.__ergo_nodecode = {};", config.compiled);
        let set_node_state = format!("__ergo_dataflow.initState({state})");

        let worker = JsWorker::new();
        worker
            .run(|runtime| {
                async move {
                    runtime.execute_script("task_environment", DATAFLOW_ENV_CODE)?;
                    runtime.execute_script("node_state", &set_node_state)?;
                    runtime.execute_script("node_code", &code)?;
                    Ok::<(), ergo_js::Error>(())
                }
                .boxed_local()
            })
            .await
            .map_err(|e| Error::DataflowInitScriptError { error: e })?;

        Ok(DataFlowRunner { worker })
    }

    /// Run a node and return the result, serialized for storage.
    pub async fn run_node(
        &self,
        task_name: &str,
        node_name: &str,
        node_func: &str,
        null_check_nodes: &[&str],
    ) -> Result<(String, Vec<ConsoleMessage>), Error> {
        let name = format!("https://ergo/tasks/{task_name}/{node_name}.js");
        let null_checks = if null_check_nodes.is_empty() {
            "null".to_string()
        } else {
            serde_json::to_string(null_check_nodes)?
        };

        let func_call = format!(
            r##"__ergo_dataflow.runNode("{node_name}", "__ergo_nodecode", "{node_func}", {null_checks})"##
        );

        self.worker
            .run(move |runtime| {
                async move {
                    let run_result = runtime.await_expression::<String>(&name, &func_call).await;
                    let console = runtime.take_console_messages();
                    match run_result {
                        Ok(value) => Ok((value, console)),
                        // TODO Generate a source map and use it to translate the code locations in the error.
                        Err(error) => Err(Error::TaskScript { error, console }),
                    }
                }
                .boxed_local()
            })
            .await
    }

    /// Set a node's state and return the serialized version of the state.
    pub async fn set_node_state(
        &self,
        node_name: &str,
        value: &serde_json::Value,
    ) -> Result<String> {
        let json = serde_json::to_string(value)?;
        let code = format!(r##"__ergo_dataflow.setNodeState("{node_name}", {json})"##);

        self.worker
            .run(move |runtime| {
                async move { runtime.run_expression::<String>("set_node_state", &code) }
                    .boxed_local()
            })
            .await
            .map_err(|e| Error::DataflowSetStateError {
                node: node_name.to_string(),
                error: e,
            })
    }

    pub async fn get_raw_state(
        &self,
        node_name: &str,
    ) -> Result<serde_json::Value, ergo_js::Error> {
        let code = format!(r##"__ergo_dataflow.getState("{node_name}")"##);
        self.worker
            .run(|runtime| {
                async move { runtime.run_expression::<serde_json::Value>("get_state", &code) }
                    .boxed_local()
            })
            .await
    }

    pub async fn retrieve_state(&self) -> Result<FxHashMap<String, String>, ergo_js::Error> {
        self.worker
            .run(|runtime| {
                async move {
                    runtime.run_expression::<FxHashMap<String, String>>(
                        "retrieve_state",
                        "__ergo_dataflow.serializeState()",
                    )
                }
                .boxed_local()
            })
            .await
    }
}
