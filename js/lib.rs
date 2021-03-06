mod console;
pub mod module_loader;
pub mod permissions;
mod pool;
mod raw_serde;
pub mod serialized_execution;

pub use console::*;
pub use pool::RuntimePool;
pub use serialized_execution::SerializedState;

pub use deno_core::{Extension, Snapshot};

use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use deno_core::{error::AnyError, op_sync, JsRuntime, OpState};
use deno_web::BlobStore;
use rusty_v8 as v8;
use serde::{de::DeserializeOwned, Serialize};
use serde_v8::{from_v8, to_v8};

use crate::permissions::Permissions;

/// Core extensions and extensions to allow network access.
pub fn net_extensions(crypto_seed: Option<u64>) -> Vec<Extension> {
    vec![
        deno_console::init(),
        deno_webidl::init(),
        deno_url::init(),
        deno_web::init(BlobStore::default(), None),
        deno_crypto::init(crypto_seed),
        deno_net::init::<Permissions>(None, false, None),
        deno_fetch::init::<Permissions>("ergo".to_string(), None, None, None, None),
    ]
}

/// A basic set of extensions for console access and URL parsing.
pub fn core_extensions(crypto_seed: Option<u64>) -> Vec<Extension> {
    vec![
        deno_console::init(),
        deno_webidl::init(),
        deno_url::init(),
        deno_web::init(BlobStore::default(), None),
        deno_crypto::init(crypto_seed),
    ]
}

pub struct RuntimeOptions {
    pub will_snapshot: bool,
    /// Deno extensions to pass to the runtime. If using serialized state,
    /// the deno_crypto extension should be initialized with the random_seed
    /// value from the SerializedState.
    pub extensions: Vec<Extension>,
    pub snapshot: Option<Snapshot>,

    /// Serialized event state for this isolate. If None, serialized execution (including saving
    /// results) is entirely disabled. To use serial execution without some existing
    /// state, set this to Some(SerializedState::default()).
    pub serialized_state: Option<SerializedState>,

    pub console: Option<Box<dyn Console>>,

    /// Permissions for Javascript code.
    pub permissions: Option<Permissions>,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        RuntimeOptions {
            will_snapshot: false,
            extensions: net_extensions(None),
            snapshot: None,
            serialized_state: None,
            console: None,
            permissions: None,
        }
    }
}

pub struct Runtime {
    runtime: JsRuntime,
}

impl Deref for Runtime {
    type Target = JsRuntime;

    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}

impl DerefMut for Runtime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.runtime
    }
}

impl Runtime {
    pub fn new(mut options: RuntimeOptions) -> Self {
        if let Some(console) = options.console {
            options.extensions.push(console_extension(console));
        }

        let has_snapshot = options.snapshot.is_some();
        let deno_runtime = JsRuntime::new(deno_core::RuntimeOptions {
            will_snapshot: options.will_snapshot,
            extensions: options.extensions,
            startup_snapshot: options.snapshot,
            ..deno_core::RuntimeOptions::default()
        });

        let mut runtime = Runtime {
            runtime: deno_runtime,
        };

        runtime
            .op_state()
            .borrow_mut()
            .put(options.permissions.unwrap_or_else(Default::default));

        if !has_snapshot {
            // If we have a snapshot, then this will already have run. If not, then run it now.
            runtime
                .execute_script("<startup>", include_str!("startup.js"))
                .expect("Running startup code");
        }

        if let Some(state) = options.serialized_state {
            if options.will_snapshot {
                // This requires setting external references in the V8 runtime and that API is
                // not currently exposed from deno_core, which uses its own fixed set of references.
                // You can still create a snapshot and then later load that snapshot with
                // serialized execution enabled.
                panic!("Serialized execution is not supported when will_snapshot is true.");
            }

            runtime.install_serialized_execution(state);
        }

        runtime
    }

    /// Retrieve the current set of console messages from the runtime.
    /// This only really does anything for a [BufferConsole], since other console
    /// implementations don't save their messages.
    pub fn take_console_messages(&mut self) -> Vec<ConsoleMessage> {
        match self
            .runtime
            .op_state()
            .borrow_mut()
            .try_borrow_mut::<ConsoleWrapper>()
        {
            Some(console) => console.console.take_messages(),
            None => Vec::new(),
        }
    }

    pub fn make_snapshot(&mut self) -> Vec<u8> {
        let snapshot = self.runtime.snapshot();
        snapshot.as_ref().to_vec()
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
        // Convert to a Local handle to work with from_v8.
        let local = v8::Local::new(&mut scope, result);
        let value = from_v8(&mut scope, local)?;
        Ok(value)
    }

    pub fn run_boolean_expression<T: Serialize>(
        &mut self,
        name: &str,
        value: &T,
        expression: &str,
    ) -> Result<bool, AnyError> {
        let script = Self::safe_braces(expression);
        self.set_global_value("value", value)?;

        let result = self.runtime.execute_script(name, &script)?;
        let mut scope = self.runtime.handle_scope();
        let local = result.get(&mut scope);
        Ok(local.boolean_value(&mut scope))
    }

    pub fn set_global_value<T: Serialize>(&mut self, key: &str, value: &T) -> Result<(), AnyError> {
        let mut scope = self.runtime.handle_scope();
        let jskey = v8::String::new(&mut scope, key).unwrap();
        let value = to_v8(&mut scope, value)?;

        let global = scope.get_current_context().global(&mut scope);
        global.set(&mut scope, jskey.into(), value);
        Ok(())
    }

    pub fn get_global_value<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, serde_v8::Error> {
        let mut scope = self.runtime.handle_scope();
        let global = scope.get_current_context().global(&mut scope);
        let jskey = v8::String::new(&mut scope, key).unwrap();
        let v8_value = global.get(&mut scope, jskey.into());
        v8_value.map(|v| from_v8(&mut scope, v)).transpose()
    }

    pub fn get_value_at_path<'a, S: AsRef<str>>(
        scope: &'a mut v8::HandleScope<'a>,
        path: &[S],
    ) -> Option<v8::Local<'a, v8::Value>> {
        let mut scope = v8::EscapableHandleScope::new(scope);
        let global = scope.get_current_context().global(&mut scope);

        let mut object: v8::Local<v8::Value> = global.into();
        for key in path {
            let s = v8::String::new(&mut scope, key.as_ref()).unwrap();
            match object
                .to_object(&mut scope)
                .and_then(|obj| obj.get(&mut scope, s.into()))
            {
                Some(o) => object = o,
                None => return None,
            };
        }

        Some(scope.escape(object))
    }
}

fn op_console(state: &mut OpState, message: String, level: usize) -> Result<(), AnyError> {
    if let Some(console) = state.try_borrow_mut::<ConsoleWrapper>() {
        let message = console::ConsoleMessage {
            message,
            level: ConsoleLevel::from(level),
            time: chrono::Utc::now(),
        };

        console.console.add(message);
    }

    Ok(())
}

struct ConsoleWrapper {
    console: Box<dyn Console>,
}

const CONSOLE_EXTENSION_JS: &'static str = r##"
    globalThis.console = new globalThis.__bootstrap.console.Console(
        (level, message) => Deno.core.opSync("ergo_js_console", level, message)
    );"##;

fn console_extension(console: Box<dyn Console>) -> deno_core::Extension {
    deno_core::Extension::builder()
        .js(vec![(
            "ergo_js_console",
            Box::new(|| Ok(String::from(CONSOLE_EXTENSION_JS))),
        )])
        .ops(vec![("ergo_js_console", op_sync(op_console))])
        .state(move |state| {
            state.put(ConsoleWrapper {
                console: console.clone_settings(),
            });
            Ok(())
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_extensions_init() {
        Runtime::new(RuntimeOptions {
            extensions: super::core_extensions(None),
            ..Default::default()
        });
    }

    #[test]
    fn net_extensions_init() {
        Runtime::new(RuntimeOptions {
            extensions: super::net_extensions(None),
            ..Default::default()
        });
    }

    mod run_expression {
        use super::*;
        use serde::Deserialize;
        use serde_json::json;

        #[test]
        fn simple_expression() {
            let mut runtime = Runtime::new(RuntimeOptions {
                will_snapshot: false,
                ..Default::default()
            });
            let value = runtime
                .run_expression::<u32>("test_add", "3 + 4")
                .expect("Ok");
            assert_eq!(value, 7);
        }

        #[test]
        fn returns_object() {
            let mut runtime = Runtime::new(RuntimeOptions {
                will_snapshot: false,
                ..Default::default()
            });
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

            let mut runtime = Runtime::new(RuntimeOptions {
                will_snapshot: false,
                ..Default::default()
            });
            let input = InputValue { a: 5 };
            runtime.set_global_value("x", &input).unwrap();

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
            let mut runtime = Runtime::new(RuntimeOptions {
                will_snapshot: false,
                ..Default::default()
            });
            let result = runtime
                .run_boolean_expression("script", &5, "value === 5")
                .unwrap();
            assert_eq!(result, true, "value === 5 where value is 5");

            let result = runtime
                .run_boolean_expression("script", &1, "value > 2")
                .unwrap();
            assert_eq!(result, false, "value > 2 where value is 1");
        }

        #[test]
        fn object() {
            let mut runtime = Runtime::new(RuntimeOptions {
                will_snapshot: false,
                ..Default::default()
            });
            let test_value = json!({
                "x": {"y": 1 }
            });
            let result = runtime
                .run_boolean_expression("script", &test_value, "value.x.y === 1")
                .unwrap();
            assert_eq!(result, true, "comparison passed");
        }
    }
}
