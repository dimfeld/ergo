#![allow(clippy::bool_assert_comparison)]

mod console;
pub mod module_loader;
pub mod permissions;
mod pool;
mod raw_serde;
#[cfg(feature = "serialized_execution")]
pub mod serialized_execution;

pub use console::*;
pub use pool::RuntimePool;
#[cfg(feature = "serialized_execution")]
pub use serialized_execution::SerializedState;

pub use deno_core::{Extension, Snapshot};
use url::Url;

use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use deno_core::{error::AnyError, op, JsRuntime, OpState};
use deno_web::BlobStore;
use serde::{de::DeserializeOwned, Serialize};
use serde_v8::{from_v8, to_v8};
use thiserror::Error;

use crate::permissions::Permissions;

pub enum RetrievedV8Value<'s> {
    Value(v8::Local<'s, v8::Value>),
    Error(v8::Local<'s, v8::Value>),
    Promise(v8::Local<'s, v8::Promise>),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("The value is a promise that failed to resolve")]
    UnresolvedPromise,
    #[error("The promise was rejected")]
    RejectedPromise(deno_core::error::JsError),
    #[error("Failed to deserialize value")]
    Deserialize(#[from] serde_v8::Error),
    #[error("JS error: {0}")]
    Runtime(#[from] deno_core::error::AnyError),
}

impl Error {
    fn rejected_promise(scope: &mut v8::HandleScope, v: v8::Local<v8::Value>) -> Self {
        let js_error = deno_core::error::JsError::from_v8_exception(scope, v);
        Error::RejectedPromise(js_error)
    }
}

// This is done as a macro so that Rust can reuse the borrow on the scope,
// instead of treating the returned value's reference to the scope as a new mutable borrow.
macro_rules! extract_promise {
    ($scope: expr, $v: expr) => {
        // If it's a promise, try to get the value out.
        if $v.is_promise() {
            let promise = v8::Local::<v8::Promise>::try_from($v).unwrap();
            match promise.state() {
                v8::PromiseState::Pending => RetrievedV8Value::Promise(promise),
                v8::PromiseState::Fulfilled => RetrievedV8Value::Value(promise.result(&mut $scope)),
                v8::PromiseState::Rejected => RetrievedV8Value::Error(promise.result(&mut $scope)),
            }
        } else {
            RetrievedV8Value::Value($v)
        }
    };
}

/// Core extensions and extensions to allow network access.
pub fn net_extensions(crypto_seed: Option<u64>) -> Vec<Extension> {
    vec![
        deno_webidl::init(),
        deno_console::init(),
        deno_url::init(),
        deno_tls::init(),
        deno_web::init::<Permissions>(BlobStore::default(), None),
        deno_crypto::init(crypto_seed),
        deno_fetch::init::<Permissions>(deno_fetch::Options {
            user_agent: "ergo".to_string(),
            ..Default::default()
        }),
        deno_net::init::<Permissions>(None, false, None),
    ]
}

/// A basic set of extensions for console access and URL parsing.
pub fn core_extensions(crypto_seed: Option<u64>) -> Vec<Extension> {
    vec![
        deno_console::init(),
        deno_webidl::init(),
        deno_url::init(),
        deno_tls::init(),
        deno_web::init::<Permissions>(BlobStore::default(), None),
        deno_crypto::init(crypto_seed),
    ]
}

pub struct RuntimeOptions {
    pub will_snapshot: bool,
    /// Deno extensions to pass to the runtime. If using serialized state,
    /// the deno_crypto extension should be initialized with the random_seed
    /// value from the SerializedState.
    pub extensions: Vec<Extension>,
    pub allow_timers: bool,
    pub ignore_unhandled_promise_rejections: bool,
    pub snapshot: Option<Snapshot>,

    #[cfg(feature = "serialized_execution")]
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
            allow_timers: true,
            ignore_unhandled_promise_rejections: false,
            snapshot: None,
            #[cfg(feature = "serialized_execution")]
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
        let console = options
            .console
            .unwrap_or_else(|| Box::new(NullConsole::new()));
        options.extensions.push(console_extension(console));

        let has_snapshot = options.snapshot.is_some();
        let deno_runtime = JsRuntime::new(deno_core::RuntimeOptions {
            will_snapshot: options.will_snapshot,
            extensions: Vec::new(),
            extensions_with_js: options.extensions,
            startup_snapshot: options.snapshot,
            module_loader: Some(Rc::new(module_loader::TrivialModuleLoader {})),
            ..deno_core::RuntimeOptions::default()
        });

        let mut runtime = Runtime {
            runtime: deno_runtime,
        };

        runtime
            .op_state()
            .borrow_mut()
            .put(options.permissions.unwrap_or_default());

        if !options.allow_timers {
            runtime
                .set_global_value("__allowTimers", &false)
                .expect("Failed to set __allowTimers");
        }

        if options.ignore_unhandled_promise_rejections {
            runtime
                .set_global_value("__handleUncaughtPromiseRejections", &false)
                .expect("Failed to set __handleUncaughtPromiseRejections");
        }

        if !has_snapshot {
            // If we have a snapshot, then this will already have run. If not, then run it now.
            runtime
                .execute_script("<startup>", include_str!("startup.js"))
                .expect("Running startup code");
        }

        if !options.will_snapshot {
            runtime
                .execute_script(
                    "<startup_postsnapshot>",
                    include_str!("startup_postsnapshot.js"),
                )
                .expect("Running startup code");
        }

        #[cfg(feature = "serialized_execution")]
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

    pub fn make_snapshot(self) -> Vec<u8> {
        let snapshot = self.runtime.snapshot();
        snapshot.as_ref().to_vec()
    }

    /// Run an expression and return the value
    pub fn run_expression<T: DeserializeOwned>(
        &mut self,
        name: &str,
        script: &str,
    ) -> Result<T, AnyError> {
        let result = self.runtime.execute_script(name, script)?;
        let mut scope = self.runtime.handle_scope();
        // Convert to a Local handle to work with from_v8.
        let local = v8::Local::new(&mut scope, result);
        let value = from_v8(&mut scope, local)?;
        Ok(value)
    }

    /// Run an expression. If it returns a promise, wait for that promise to resolve, then return the value.
    pub async fn await_expression<T: DeserializeOwned>(
        &mut self,
        name: &str,
        script: &str,
    ) -> Result<T, Error> {
        let result = self.runtime.execute_script(name, script)?;

        {
            let mut scope = self.runtime.handle_scope();
            let local = v8::Local::new(&mut scope, &result);
            let result = extract_promise!(&mut scope, local);
            match result {
                RetrievedV8Value::Value(v) => return from_v8(&mut scope, v).map_err(Error::from),
                RetrievedV8Value::Error(e) => return Err(Error::rejected_promise(&mut scope, e)),
                // Try to await it below
                RetrievedV8Value::Promise(_) => {}
            }
        }

        // Wait for the promise to resolve.
        self.run_event_loop(false).await?;

        let mut scope = self.runtime.handle_scope();
        let local = v8::Local::new(&mut scope, result);
        let promise_result = extract_promise!(&mut scope, local);
        match promise_result {
            RetrievedV8Value::Value(v) => from_v8(&mut scope, v).map_err(Error::from),
            RetrievedV8Value::Error(e) => Err(Error::rejected_promise(&mut scope, e)),
            RetrievedV8Value::Promise(_) => Err(Error::UnresolvedPromise),
        }
    }

    pub fn run_boolean_expression<T: Serialize>(
        &mut self,
        name: &str,
        value: &T,
        script: &str,
    ) -> Result<bool, AnyError> {
        self.set_global_value("value", value)?;

        let result = self.runtime.execute_script(name, script)?;
        let mut scope = self.runtime.handle_scope();
        let local = result.open(&mut scope);
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

    /// Get a value without trying to deserialize it. If it's a resolved promise, extract the
    /// promise's value.
    fn get_global_raw_value(&mut self, key: &str) -> Option<(v8::HandleScope, RetrievedV8Value)> {
        let mut scope = self.runtime.handle_scope();
        let global = scope.get_current_context().global(&mut scope);
        let jskey = v8::String::new(&mut scope, key).unwrap();
        let value = global.get(&mut scope, jskey.into());

        value
            .map(|v| extract_promise!(&mut scope, v))
            .map(|v| (scope, v))
    }

    /// Retrieve a global value from the runtime. If it's a resolved promise, extract the value.
    /// Returns an error if the promise is unresolved or rejected.
    /// that they will succeed only if the Promise can deserialize into the target type.
    pub fn get_global_value<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>, Error> {
        let (mut scope, v8_value) = match self.get_global_raw_value(key) {
            Some(v) => v,
            None => return Ok(None),
        };

        match v8_value {
            RetrievedV8Value::Value(v) => from_v8(&mut scope, v).map_err(|e| e.into()),
            RetrievedV8Value::Error(e) => Err(Error::rejected_promise(&mut scope, e)),
            RetrievedV8Value::Promise(_) => Err(Error::UnresolvedPromise),
        }
    }

    /// Retrieve a global value from the runtime. If it's a resolved promise, extract the value.
    /// If it's an unresolved promise, wait for it to resolve. Returns an error for rejected
    /// promises.
    pub async fn await_global_value<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, Error> {
        {
            let value = self.get_global_raw_value(key);
            match value {
                Some((mut scope, RetrievedV8Value::Value(v))) => {
                    return from_v8(&mut scope, v).map(Some).map_err(Error::from)
                }
                Some((mut scope, RetrievedV8Value::Error(e))) => {
                    return Err(Error::rejected_promise(&mut scope, e));
                }
                Some((_, RetrievedV8Value::Promise(_))) => {
                    // Try again below. We have to drop `value` first so that's why we do it this way.
                }
                None => return Ok(None),
            }
        }

        // Run the event loop and try one more time.
        // This can be a bit more efficient by usinlg `resolve_value`.
        dbg!("running event loop");
        self.run_event_loop(false).await?;
        dbg!("ran event loop");

        match self.get_global_raw_value(key) {
            Some((mut scope, RetrievedV8Value::Value(v))) => {
                from_v8(&mut scope, v).map(Some).map_err(Error::from)
            }
            Some((mut scope, RetrievedV8Value::Error(e))) => {
                Err(Error::rejected_promise(&mut scope, e))
            }
            Some((_, RetrievedV8Value::Promise(_))) => Err(Error::UnresolvedPromise),
            None => Ok(None),
        }
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

    pub async fn run_main_module(&mut self, url: Url, source: String) -> Result<(), AnyError> {
        let mod_id = self.load_main_module(&url, Some(source)).await?;
        let mod_done = self.mod_evaluate(mod_id);
        self.run_event_loop(false).await?;
        mod_done.await?
    }
}

#[op]
fn ergo_js_console(state: &mut OpState, message: String, level: usize) -> Result<(), AnyError> {
    if let Some(console) = state.try_borrow_mut::<ConsoleWrapper>() {
        let message = console::ConsoleMessage {
            message,
            level: ConsoleLevel::from(level),
            time: chrono::Utc::now(),
        };

        console.console.add(message);
    } else {
        panic!("No console wrapper")
    }

    Ok(())
}

struct ConsoleWrapper {
    console: Box<dyn Console>,
}

const CONSOLE_EXTENSION_JS: &str = r##"
    globalThis.console = new globalThis.__bootstrap.console.Console(
        (message, level) => Deno.core.ops.ergo_js_console(message, level)
    );"##;

fn console_extension(console: Box<dyn Console>) -> deno_core::Extension {
    deno_core::Extension::builder()
        .js(vec![("ergo_js_console", CONSOLE_EXTENSION_JS)])
        .ops(vec![ergo_js_console::decl()])
        .state(move |state| {
            state.put(ConsoleWrapper {
                console: console.clone_settings(),
            });
            Ok(())
        })
        .build()
}

/// When evaluating a raw expression like { a: 5 }, V8 sees the
/// first brace as entering a scope rather than creating an object.
/// Wrapping the expressison in parentheses prevents this.
pub fn safe_braces(mut expr: &str) -> Cow<'_, str> {
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

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

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
            let expression = safe_braces("{ a: 5, b: { c: 6 } }");
            let value = runtime
                .run_expression::<serde_json::Value>("test_object", expression.as_ref())
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
                .run_expression("test", safe_braces("let a = x; { b: x.a + 1 };").as_ref())
                .expect("Value converts");
            assert_eq!(result, OutputValue { b: 6 });
        }

        #[test]
        fn iife() {
            let mut runtime = Runtime::new(RuntimeOptions {
                will_snapshot: false,
                ..Default::default()
            });

            let script = r##"(function() { return 5; })()"##;

            let result: i64 = runtime.run_expression("script", script).unwrap();
            assert_eq!(result, 5);
        }
    }

    mod await_expression {
        use super::*;

        #[tokio::test]
        async fn resolved_promise() {
            let code = r##"Promise.resolve(5);"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            let value: u32 = runtime
                .await_expression("test", code)
                .await
                .expect("running code");

            assert_eq!(value, 5);
        }

        #[tokio::test]
        async fn unresolveable() {
            let code = r##"new Promise(() => {})"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            let value = runtime
                .await_expression::<serde_json::Value>("test", code)
                .await;

            assert!(matches!(value, Err(Error::UnresolvedPromise)));
        }

        #[tokio::test]
        async fn rejected() {
            let code = r##"Promise.reject(new Error("test error"));"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            let value = runtime.await_expression::<u32>("test", code).await;

            dbg!(&value);
            assert!(matches!(
                value,
                Err(Error::RejectedPromise(x)) if x.message.as_deref().unwrap_or_default() == "test error"));
        }

        #[tokio::test]
        async fn awaits_pending_promise() {
            let code = r##"new Promise((resolve) => {
                setTimeout(() => resolve(5));
             });"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            let value: u32 = runtime
                .await_expression("test", code)
                .await
                .expect("running code");

            dbg!(&value);
            assert_eq!(value, 5);
        }

        #[tokio::test]
        async fn rejected_after_await() {
            let code = r##"new Promise((resolve, reject) => {
                setTimeout(() => reject(new Error('test error')));
             });"##;
            let mut runtime = Runtime::new(RuntimeOptions {
                ignore_unhandled_promise_rejections: true,
                ..Default::default()
            });
            let value = runtime
                .await_expression::<serde_json::Value>("test", code)
                .await;

            dbg!(&value);
            assert!(matches!(
                value,
                Err(Error::RejectedPromise(x)) if x.message.as_deref().unwrap_or_default() == "test error"));
        }
    }

    mod get_global_value {
        use super::*;
        use serde_json::json;

        #[test]
        fn resolved_promise() {
            let code = r##"globalThis.x = Promise.resolve(5);"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            runtime
                .run_expression::<serde_json::Value>("test", code)
                .expect("running code");
            let value: u32 = runtime
                .get_global_value("x")
                .expect("retrieving value")
                .expect("value exists");

            assert_eq!(value, 5);
        }

        #[test]
        fn unresolveable() {
            let code = r##"globalThis.x = new Promise(() => {})"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            runtime
                .run_expression::<serde_json::Value>("test", code)
                .expect("running code");
            let value = runtime.get_global_value::<u32>("x");

            assert!(matches!(value, Err(Error::UnresolvedPromise)));
        }

        #[tokio::test]
        async fn needs_resolving() {
            let code = r##"globalThis.x = new Promise((resolve) => {
                setTimeout(() => resolve(5));
             });"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            runtime
                .run_expression::<serde_json::Value>("test", code)
                .expect("running code");
            let value = runtime.get_global_value::<u32>("x");

            // `get_global_value` doesn't try to resolve this promise, it just returns an error.
            dbg!(&value);
            assert!(matches!(value, Err(Error::UnresolvedPromise)));
        }

        #[test]
        fn rejected() {
            let code = r##"globalThis.x = Promise.reject(new Error("test error"));"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            runtime
                .run_expression::<serde_json::Value>("test", code)
                .expect("running code");
            let value = runtime.get_global_value::<u32>("x");
            dbg!(&value);
            assert!(matches!(
                value,
                Err(Error::RejectedPromise(x)) if x.message.as_deref().unwrap_or_default() == "test error"));
        }
    }

    mod await_global_value {
        use super::*;

        #[tokio::test]
        async fn resolved_promise() {
            let code = r##"globalThis.x = Promise.resolve(5);"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            runtime
                .run_expression::<serde_json::Value>("test", code)
                .expect("running code");
            let value: u32 = runtime
                .await_global_value("x")
                .await
                .expect("retrieving value")
                .expect("value exists");

            assert_eq!(value, 5);
        }

        #[tokio::test]
        async fn unresolveable() {
            let code = r##"globalThis.x = new Promise(() => {})"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            runtime
                .run_expression::<serde_json::Value>("test", code)
                .expect("running code");
            let value = runtime.await_global_value::<u32>("x").await;

            assert!(matches!(value, Err(Error::UnresolvedPromise)));
        }

        #[tokio::test]
        async fn rejected() {
            let code = r##"globalThis.x = Promise.reject(new Error("test error"));"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            runtime
                .run_expression::<serde_json::Value>("test", code)
                .expect("running code");
            let value = runtime.await_global_value::<u32>("x").await;
            dbg!(&value);
            assert!(matches!(
                value,
                Err(Error::RejectedPromise(x)) if x.message.as_deref().unwrap_or_default() == "test error"));
        }

        #[tokio::test]
        async fn awaits_pending_promise() {
            let code = r##"globalThis.x = new Promise((resolve) => {
                setTimeout(() => resolve(5));
             });"##;
            let mut runtime = Runtime::new(RuntimeOptions::default());
            runtime
                .run_expression::<serde_json::Value>("test", code)
                .expect("running code");
            let value = runtime
                .await_global_value::<u32>("x")
                .await
                .expect("retrieving value")
                .expect("value is present");

            dbg!(&value);
            assert_eq!(value, 5);
        }

        #[tokio::test]
        async fn rejected_after_await() {
            let code = r##"globalThis.x = new Promise((resolve, reject) => {
                setTimeout(() => reject(new Error('test error')));
             });"##;
            let mut runtime = Runtime::new(RuntimeOptions {
                ignore_unhandled_promise_rejections: true,
                ..Default::default()
            });
            runtime
                .run_expression::<serde_json::Value>("test", code)
                .expect("running code");
            let value = runtime.await_global_value::<u32>("x").await;

            dbg!(&value);
            assert!(matches!(
                value,
                Err(Error::RejectedPromise(x)) if x.message.as_deref().unwrap_or_default() == "test error"));
        }
    }

    mod run_boolean_expression {
        use super::*;
        use serde_json::json;

        #[test]
        fn simple() {
            let mut runtime = Runtime::new(RuntimeOptions::default());
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
            let mut runtime = Runtime::new(RuntimeOptions::default());
            let test_value = json!({
                "x": {"y": 1 }
            });
            let result = runtime
                .run_boolean_expression("script", &test_value, "value.x.y === 1")
                .unwrap();
            assert_eq!(result, true, "comparison passed");
        }
    }

    #[tokio::test]
    async fn run_main_module() {
        let mut runtime = Runtime::new(RuntimeOptions {
            extensions: core_extensions(None),
            ..Default::default()
        });
        let code = r##"
            // Top-level await just to make sure it works.
            await Promise.resolve();
            globalThis.loaded = true;
            "##;
        let main_url = Url::parse("https://ergo/script").expect("creating url");
        runtime
            .run_main_module(main_url, code.to_string())
            .await
            .expect("run_main_module");
        let loaded: bool = runtime
            .get_global_value("loaded")
            .expect("retrieving result")
            .expect("result should be present");
        assert_eq!(loaded, true, "loaded is true");
    }

    #[tokio::test]
    async fn fetch() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "a": 5 })))
            .mount(&server)
            .await;

        let mut runtime = Runtime::new(RuntimeOptions {
            extensions: net_extensions(None),
            console: Some(Box::new(BufferConsole::new(ConsoleLevel::Info))),
            ..Default::default()
        });

        let script = format!(
            r##"
            let result = await fetch("{}").then((r) => r.json());
            console.dir(result);
            globalThis.result = result;
        "##,
            server.uri()
        );

        let script_result = runtime
            .run_main_module(Url::parse("https://ergo/script").unwrap(), script)
            .await;
        println!("Console: {:?}", runtime.take_console_messages());
        script_result.expect("running script");
        let result: serde_json::Value = runtime
            .get_global_value("result")
            .expect("getting result")
            .expect("getting_result");
        assert_eq!(result, json!({"a": 5}));
    }
}
