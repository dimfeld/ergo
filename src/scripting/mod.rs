use deno_core::{JsRuntime, RuntimeOptions};
use rusty_v8::StartupData;

pub struct Snapshot(Box<[u8]>);

impl Clone for Snapshot {
    fn clone(&self) -> Self {
        Snapshot(self.0.clone())
    }
}

pub struct Runtime {
    runtime: JsRuntime,
}

impl Runtime {
    pub fn new() -> Self {
        let runtime = JsRuntime::new(RuntimeOptions::default());
        Runtime { runtime }
    }

    pub fn for_snapshot() -> Self {
        let runtime = JsRuntime::new(RuntimeOptions {
            will_snapshot: true,
            ..Default::default()
        });

        Runtime { runtime }
    }

    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        let runtime = JsRuntime::new(RuntimeOptions {
            startup_snapshot: Some(deno_core::Snapshot::Boxed(snapshot.clone().0)),
            ..Default::default()
        });

        Runtime { runtime }
    }

    pub fn make_snapshot(&mut self) -> Snapshot {
        let snapshot = self.runtime.snapshot();
        Snapshot(snapshot.as_ref().to_vec().into_boxed_slice())
    }
}
