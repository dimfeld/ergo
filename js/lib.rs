use deno_core::{error::AnyError, JsRuntime, RuntimeOptions};
use rusty_v8 as v8;
use rusty_v8::{Global, ObjectTemplate, Value};
use serde::{Deserialize, Serialize};
use serde_v8::{from_v8, to_v8};
use v8::Handle;

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

    pub fn run_boolean_expression<T: Serialize>(
        &mut self,
        value: &T,
        expression: &str,
    ) -> Result<bool, AnyError> {
        self.insert_global_object("value", value);

        let result = self
            .runtime
            .execute_script("boolean expression", expression)?;
        let local = result.get(&mut self.runtime.v8_isolate());
        let mut scope = self.runtime.handle_scope();
        Ok(local.boolean_value(&mut scope))
    }

    pub fn insert_global_object<T: Serialize>(&mut self, key: &str, value: &T) {
        let mut scope = self.runtime.handle_scope();
        let jskey = v8::String::new(&mut scope, key).unwrap();
        let value = to_v8(&mut scope, value).unwrap();

        let global = scope.get_current_context().global(&mut scope);
        global.set(&mut scope, jskey.into(), value);
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

    #[test]
    fn boolean_expression() {
        let mut runtime = Runtime::new();
        let result = runtime.run_boolean_expression(&5, "value === 5").unwrap();
        assert_eq!(result, true, "value === 5 where value is 5");

        let result = runtime.run_boolean_expression(&1, "value > 2").unwrap();
        assert_eq!(result, false, "value > 2 where value is 1");
    }
}
