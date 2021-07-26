use deno_core::{error::AnyError, JsRuntime, RuntimeOptions};
use rusty_v8::{Global, Value};
use serde_v8::{from_v8, to_v8};

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

    /** Run an expression and return the value */
    pub fn run_expression(&mut self, name: &str, text: &str) -> Result<f64, AnyError> {
        let result = self.runtime.execute_script(name, text)?;
        let mut isolate = self.runtime.v8_isolate();
        let local = result.get(&mut isolate);
        let mut scope = self.runtime.handle_scope();
        let value = local.number_value(&mut scope).unwrap();
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_expression() {
        let mut runtime = Runtime::new();
        let value = runtime.run_expression("test_add", "3 + 4").expect("Ok");
        assert_eq!(value, 7.0);
    }
}
