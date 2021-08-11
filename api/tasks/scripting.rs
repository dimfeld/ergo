use ergo_js::{BufferConsole, Extension, Runtime, SerializedState, Snapshot};

const NET_SNAPSHOT: &'static [u8] = include_bytes!("../snapshots/net");
const CORE_SNAPSHOT: &'static [u8] = include_bytes!("../snapshots/core");

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

    ergo_js::Runtime::new(ergo_js::RuntimeOptions {
        console: Some(Box::new(BufferConsole::new(ergo_js::ConsoleLevel::Debug))),
        extensions,
        snapshot: Some(Snapshot::Static(snapshot)),
        serialized_state: Some(state),
        ..Default::default()
    })
}

/// Create a simple runtime without net access or serialized execution.
pub fn create_simple_runtime() -> Runtime {
    ergo_js::Runtime::new(ergo_js::RuntimeOptions {
        console: Some(Box::new(BufferConsole::new(ergo_js::ConsoleLevel::Debug))),
        extensions: ergo_js::core_extensions(None),
        snapshot: Some(Snapshot::Static(CORE_SNAPSHOT)),
        ..Default::default()
    })
}
