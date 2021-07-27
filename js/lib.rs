use std::borrow::Cow;

use deno_core::{error::AnyError, JsRuntime, RuntimeOptions};
use rusty_v8 as v8;
use rusty_v8::{Global, ObjectTemplate, Value};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_v8::{from_v8, to_v8};
use v8::{Handle, Local};

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

    /// When evaluating a raw expression like { a: 5 }, V8 sees the
    /// first brace as entering a scope rather than creating an object.
    /// Wrapping the expressison in parentheses prevents this.
    fn safe_braces<'a>(mut expr: &'a str) -> Cow<'a, str> {
        expr = expr.trim();

        // Handle expressions that end in ;
        if expr.ends_with(';') {
            expr = &expr[0..expr.len() - 1];
        }

        // Get the final expression and wrap it if necessary.
        let splits = expr.rsplit_once(';');
        match splits {
            Some((first, last)) => {
                let l = last.trim();
                if l.starts_with('{') && l.ends_with('}') {
                    Cow::from(format!("{}; ({})", first, l))
                } else {
                    Cow::from(expr)
                }
            }
            None => {
                let trimmed = expr.trim();
                if trimmed.starts_with('{') && trimmed.ends_with('}') {
                    Cow::from(format!("({})", expr))
                } else {
                    Cow::from(expr)
                }
            }
        }
    }

    /** Run an expression and return the value */
    pub fn run_expression<T: DeserializeOwned>(
        &mut self,
        name: &str,
        text: &str,
    ) -> Result<T, AnyError> {
        let script = Self::safe_braces(text);
        let result = self.runtime.execute_script(name, &script)?;
        let mut scope = self.runtime.handle_scope();
        let local = Local::new(&mut scope, result);

        println!(
            "{}",
            v8::json::stringify(&mut scope, local.clone())
                .unwrap()
                .to_rust_string_lossy(&mut scope)
        );

        let value = from_v8(&mut scope, local).unwrap();
        Ok(value)
    }

    pub fn run_boolean_expression<T: Serialize>(
        &mut self,
        value: &T,
        expression: &str,
    ) -> Result<bool, AnyError> {
        let script = Self::safe_braces(expression);
        self.insert_global_value("value", value);

        let result = self.runtime.execute_script("boolean expression", &script)?;
        let mut scope = self.runtime.handle_scope();
        let local = result.get(&mut scope);
        Ok(local.boolean_value(&mut scope))
    }

    pub fn insert_global_value<T: Serialize>(&mut self, key: &str, value: &T) {
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

    mod run_expression {
        use super::*;
        use serde_json::json;

        #[test]
        fn simple_expression() {
            let mut runtime = Runtime::new();
            let value = runtime
                .run_expression::<u32>("test_add", "3 + 4")
                .expect("Ok");
            assert_eq!(value, 7);
        }

        #[test]
        fn returns_object() {
            let mut runtime = Runtime::new();
            let expression = "{ a: 5, b: { c: 6 } }";
            let value = runtime
                .run_expression::<serde_json::Value>("test_object", expression)
                .expect("Ok");
            assert_eq!(value, json!({ "a": 5, "b": { "c": 6 } }));
        }

        #[test]
        fn strong_typing() {
            #[derive(Serialize)]
            struct InputValue {
                a: u32,
            }

            #[derive(Debug, Deserialize, PartialEq, Eq)]
            struct OutputValue {
                b: u32,
            }

            let mut runtime = Runtime::new();
            let input = InputValue { a: 5 };
            runtime.insert_global_value("x", &input);

            let result: OutputValue = runtime
                .run_expression("test", "let a = x; { b: x.a + 1 };")
                .expect("Value converts");
            assert_eq!(result, OutputValue { b: 6 });
        }
    }

    mod run_boolean_expression {
        use super::*;
        use serde_json::json;

        #[test]
        fn simple() {
            let mut runtime = Runtime::new();
            let result = runtime.run_boolean_expression(&5, "value === 5").unwrap();
            assert_eq!(result, true, "value === 5 where value is 5");

            let result = runtime.run_boolean_expression(&1, "value > 2").unwrap();
            assert_eq!(result, false, "value > 2 where value is 1");
        }

        #[test]
        fn object() {
            let mut runtime = Runtime::new();
            let test_value = json!({
                "x": {"y": 1 }
            });
            let result = runtime
                .run_boolean_expression(&test_value, "value.x.y === 1")
                .unwrap();
            assert_eq!(result, true, "comparison passed");
        }
    }
}
