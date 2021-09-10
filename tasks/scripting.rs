use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "full")]
pub use full::*;

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize)]
pub struct TaskJsConfig {
    pub script: String,
}

#[cfg(feature = "full")]
mod full {
    use std::borrow::Cow;

    use ergo_js::{
        BufferConsole, ConsoleMessage, Extension, Runtime, RuntimeOptions, RuntimePool,
        SerializedState, Snapshot,
    };
    use itertools::Itertools;
    use schemars::JsonSchema;
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use tracing::{event, Level};

    const NET_SNAPSHOT: &'static [u8] = include_bytes!("./snapshots/net");
    const CORE_SNAPSHOT: &'static [u8] = include_bytes!("./snapshots/core");

    lazy_static::lazy_static! {
        pub static ref POOL : RuntimePool = RuntimePool::new(None);
    }

    #[derive(Clone, Debug, JsonSchema, Serialize, Deserialize)]
    pub struct TaskJsState {
        pub state: ergo_js::SerializedState,
        pub console: Vec<ConsoleMessage>,
    }

    fn snapshot_and_extensions(
        allow_net: bool,
        random_seed: Option<u64>,
    ) -> (&'static [u8], Vec<Extension>) {
        if allow_net {
            (NET_SNAPSHOT, ergo_js::net_extensions(random_seed))
        } else {
            (CORE_SNAPSHOT, ergo_js::core_extensions(random_seed))
        }
    }

    /// Create a runtime suitable for running tasks, with serialized execution and optional network
    /// access. If `state` is `None`, a new [SerializedState] will be created.
    pub fn create_task_script_runtime(state: Option<SerializedState>, allow_net: bool) -> Runtime {
        let state = state.unwrap_or_else(Default::default);
        let (snapshot, extensions) = snapshot_and_extensions(allow_net, Some(state.random_seed));

        Runtime::new(RuntimeOptions {
            console: Some(Box::new(BufferConsole::new(ergo_js::ConsoleLevel::Debug))),
            extensions,
            snapshot: Some(Snapshot::Static(snapshot)),
            serialized_state: Some(state),
            ..Default::default()
        })
    }

    /// Create a full-features, non-serialized runtime.
    pub fn create_executor_runtime() -> Runtime {
        let (snapshot, extensions) = snapshot_and_extensions(true, None);
        Runtime::new(RuntimeOptions {
            console: Some(Box::new(BufferConsole::new(ergo_js::ConsoleLevel::Info))),
            extensions,
            snapshot: Some(Snapshot::Static(snapshot)),
            ..Default::default()
        })
    }

    /// Create a simple runtime without net access or serialized execution.
    /// This is used for things like evaluating guard conditions in state machines.
    pub fn create_simple_runtime() -> Runtime {
        Runtime::new(RuntimeOptions {
            console: Some(Box::new(BufferConsole::new(ergo_js::ConsoleLevel::Debug))),
            extensions: ergo_js::core_extensions(None),
            snapshot: Some(Snapshot::Static(CORE_SNAPSHOT)),
            ..Default::default()
        })
    }

    pub fn wrap_in_function(script: &str) -> String {
        format!(
            r##"(function() {{
        {}
    }})()"##,
            script
        )
    }

    pub fn wrap_in_function_with_args(
        script: &str,
        arg_name: &str,
        arg: impl Serialize,
    ) -> Result<String, serde_json::Error> {
        let arg_value = serde_json::to_string(&arg)?;
        let output = format!(
            r##"(function({arg_name}) {{
        {script}
    }})({arg_value})"##,
            script = script,
            arg_name = arg_name,
            arg_value = arg_value
        );

        Ok(output)
    }

    pub async fn run_simple_with_context_and_payload<
        RESULT: DeserializeOwned + std::fmt::Debug + Send + 'static,
    >(
        script: &str,
        context: Option<&serde_json::Value>,
        payload: Option<&serde_json::Value>,
    ) -> Result<RESULT, anyhow::Error> {
        let payload_arg = payload
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Owned(serde_json::Value::Null));
        let context_arg = context
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Owned(serde_json::Value::Null));

        run_simple_with_args(
            script,
            &[
                ("context", context_arg.as_ref()),
                ("payload", payload_arg.as_ref()),
            ],
        )
        .await
    }

    pub async fn run_simple_with_args<
        RESULT: DeserializeOwned + std::fmt::Debug + Send + 'static,
    >(
        script: &str,
        args: &[(&str, &serde_json::Value)],
    ) -> Result<RESULT, anyhow::Error> {
        let wrapped = format!(
            r##"(function({arg_names}) {{
            {script}
        }})({arg_values})"##,
            arg_names = args.iter().map(|a| a.0).join(","),
            arg_values = args.iter().map(|a| a.1).join(", "),
            script = script
        );

        event!(Level::TRACE, script=%wrapped, "running script");

        POOL.run(move || async move {
            let mut runtime = create_simple_runtime();
            let result: RESULT = runtime.run_expression("script", wrapped.as_str())?;
            Ok(result)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[tokio::test]
    async fn run_simple_with_context_and_payload() {
        let input_script = r##"return payload.value"##;
        let result: i64 = super::run_simple_with_context_and_payload(
            input_script,
            None,
            Some(&json!({ "value": 5 })),
        )
        .await
        .unwrap();
        assert_eq!(result, 5);
    }
}