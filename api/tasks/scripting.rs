use std::borrow::Cow;

use ergo_js::{
    BufferConsole, Extension, Runtime, RuntimeOptions, RuntimePool, SerializedState, Snapshot,
};
use serde::{de::DeserializeOwned, Serialize};

const NET_SNAPSHOT: &'static [u8] = include_bytes!("../snapshots/net");
const CORE_SNAPSHOT: &'static [u8] = include_bytes!("../snapshots/core");

lazy_static::lazy_static! {
    pub static ref POOL : RuntimePool = RuntimePool::new(None);
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

    let wrapped = format!(
        r##"(function(payload, context) {{
            {script}
        }})({payload}, {context})"##,
        payload = payload_arg,
        context = context_arg,
        script = script
    );

    POOL.run(move || async move {
        let mut runtime = create_simple_runtime();
        let result: RESULT = runtime.run_expression("script", wrapped.as_str())?;
        Ok(result)
    })
    .await
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore]
    async fn simple_runs() {}
}
