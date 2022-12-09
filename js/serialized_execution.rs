use chrono::{DateTime, Utc};
use deno_core::error::AnyError;
use futures::future::{ready, TryFutureExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_v8::{from_v8, to_v8};
use v8::{Exception, MapFnTo};

use crate::{
    raw_serde::{self, deserialize},
    Runtime,
};

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize)]
pub struct SerializedEvent {
    /// The wall time when the event completed.
    wall_time: chrono::DateTime<Utc>,
    fn_name: String,
    args_json: Vec<serde_json::Value>,
    result: Vec<u8>,
    result_json: serde_json::Value,
}

#[derive(Debug, JsonSchema, Serialize, Deserialize, Clone)]
pub struct PendingEvent {
    pub name: String,
    pub args: Vec<serde_json::Value>,
    pub result: Option<serde_json::Value>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize)]
pub struct SerializedState {
    pub random_seed: u64,
    pub start_time: chrono::DateTime<Utc>,
    pub events: Vec<SerializedEvent>,
    pub pending: Option<PendingEvent>,
}

impl SerializedState {
    pub fn apply_pending_event(&mut self, scope: &mut v8::HandleScope) -> Result<(), AnyError> {
        let pending = match self.pending.take() {
            Some(p) => p,
            None => return Ok(()),
        };

        let wall_time = Utc::now();
        let result = pending.result.unwrap_or(serde_json::Value::Null);
        let v8_result = to_v8(scope, &result)?;
        let serialized = raw_serde::serialize(scope, v8_result)?;

        let event = SerializedEvent {
            wall_time,
            fn_name: pending.name,
            args_json: pending.args,
            result: serialized,
            result_json: result,
        };

        self.events.push(event);
        Ok(())
    }
}

impl Default for SerializedState {
    fn default() -> Self {
        SerializedState {
            random_seed: rand::random(),
            start_time: Utc::now(),
            events: Vec::new(),
            pending: None,
        }
    }
}

impl From<EventTracker> for SerializedState {
    fn from(mut e: EventTracker) -> Self {
        e.saved_results.extend(e.new_results.into_iter());
        SerializedState {
            random_seed: e.random_seed,
            start_time: e.start_time,
            events: e.saved_results,
            pending: e.pending_event,
        }
    }
}

#[derive(Debug, Clone)]
struct EventTracker {
    wall_time: DateTime<Utc>,
    saved_results: Vec<SerializedEvent>,
    next_event: usize,
    new_results: Vec<SerializedEvent>,
    no_new_results_symbol: v8::Global<v8::Symbol>,

    /// If the execution stopped because getResult was called with exitIfUnsaved
    /// and no result was found, the requested function name and arguments are
    /// stored in `pending_event`.
    pending_event: Option<PendingEvent>,

    start_time: DateTime<Utc>,
    random_seed: u64,
}

impl Runtime {
    pub fn install_serialized_execution(&mut self, history: SerializedState) {
        {
            let scope = &mut self.handle_scope();

            let jskey = v8::String::new(scope, "ErgoSerialize").unwrap();
            let ser_obj = v8::Object::new(scope);

            set_func(scope, ser_obj, "saveResult", save_result);
            set_func(scope, ser_obj, "getResult", get_result);
            set_func(scope, ser_obj, "exit", exit_call);

            let no_new_results_symbol_name =
                v8::String::new(scope, "ErgoSerialize noNewResults").unwrap();
            let no_new_results_symbol = v8::Symbol::for_global(scope, no_new_results_symbol_name);
            let key = v8::String::new(scope, "noNewResults").unwrap();
            ser_obj.set(scope, key.into(), no_new_results_symbol.into());

            let wall_time_name = v8::String::new(scope, "wallTime").unwrap();
            ser_obj.set_accessor(scope, wall_time_name.into(), wall_time_accessor);

            let global = scope.get_current_context().global(scope);
            global.set(scope, jskey.into(), ser_obj.into());

            let tracker = EventTracker {
                saved_results: history.events,
                wall_time: history.start_time,
                next_event: 0,
                new_results: Vec::new(),
                no_new_results_symbol: v8::Global::new(scope, no_new_results_symbol),
                pending_event: None,

                random_seed: history.random_seed,
                start_time: history.start_time,
            };
            scope.set_slot(tracker);
        }

        self.execute_script(
            "serialized_execution_install",
            include_str!("serialized_execution.js"),
        )
        .expect("Installing serialized execution");
    }

    /// Run with serialized execution. This pumps the event loop to completion, and also
    /// catches the execution termination exception.
    pub async fn run_serialized(
        &mut self,
        name: &str,
        script: &str,
    ) -> Result<Option<SerializedState>, AnyError> {
        let result = ready(self.runtime.execute_script(name, script))
            .and_then(|_| async { self.runtime.run_event_loop(false).await })
            .await;

        match result {
            Ok(_) => Ok(self.take_serialize_state()),
            Err(e) => {
                if e.to_string().ends_with("execution terminated") {
                    Ok(self.take_serialize_state())
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Extract the serialized state from the runtime. This clears the saved state to avoid cloning,
    /// so should only be done once this runtime is finished.
    pub fn take_serialize_state(&mut self) -> Option<SerializedState> {
        let isolate = self.runtime.v8_isolate();
        let events = isolate.get_slot_mut::<EventTracker>();

        match events {
            Some(e) => {
                // We can't just remove the slot completely, so instead replace the value with
                // an empty tracker.
                let now = Utc::now();
                let replacement = EventTracker {
                    wall_time: now,
                    saved_results: Vec::new(),
                    next_event: 0,
                    new_results: Vec::new(),
                    no_new_results_symbol: e.no_new_results_symbol.clone(),
                    pending_event: None,
                    random_seed: 0,
                    start_time: now,
                };

                Some(std::mem::replace(e, replacement).into())
            }
            None => None,
        }
    }
}

fn set_func(
    scope: &mut v8::HandleScope<'_>,
    object: v8::Local<v8::Object>,
    name: &'static str,
    func: impl MapFnTo<v8::FunctionCallback>,
) {
    let key = v8::String::new(scope, name).unwrap();
    let template = v8::FunctionTemplate::new(scope, func);
    let v8_func = template.get_function(scope).unwrap();
    object.set(scope, key.into(), v8_func.into());
}

macro_rules! get_event_state {
    ($scope: expr) => {
        match $scope.get_slot_mut::<EventTracker>() {
            Some(e) => e,
            None => {
                throw_error($scope, "Serialized execution not enabled");
                return;
            }
        }
    };
}

macro_rules! v8_try {
    ($scope:expr, $expr: expr) => {
        match $expr {
            Ok(value) => value,
            Err(e) => {
                let msg = e.to_string();
                throw_error($scope, &msg);
                return;
            }
        }
    };

    ($scope:expr, $expr: expr, $err_msg: expr) => {
        match $expr {
            Ok(value) => value,
            Err(_) => {
                throw_error($scope, $err_msg);
                return;
            }
        }
    };
}

/// Exit the script early.
fn exit_call(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    exit(scope);
}

fn exit(scope: &mut v8::HandleScope) {
    scope.terminate_execution();

    // Termination doesn't necessarily happen immediately since V8 itself only checks for
    // termination in function prologues and loops. This trick forces a terminate check,
    // suggested by the V8 team at
    // https://groups.google.com/g/v8-users/c/SpzuB-lTgcI/m/ZudO99pDXiAJ
    let last_script = v8::String::new(scope, "0").unwrap();
    let mut tc = v8::TryCatch::new(scope);
    let script = v8::Script::compile(&mut tc, last_script, None).unwrap();
    script.run(&mut tc);
}

fn save_result(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() != 3 {
        throw_error(
            scope,
            "Arguments must be the function name, its arguments, and the result",
        );
        return;
    }

    let fn_name: String = v8_try!(
        scope,
        from_v8(scope, args.get(0)),
        "First argument must be the name of the function"
    );

    let args_json: Vec<serde_json::Value> = v8_try!(
        scope,
        from_v8(scope, args.get(1)),
        "Second argument should be the list of arguments to the wrapped function"
    );
    // Save the the raw serialied result for proper reconstitution and the JSON version to
    // make it inspectable without having to fire up a V8 isolate.
    // TODO This currently fails when wrapping fetch due to handling the bytes buffer of the
    // response. I'm not fixing it for now since I'm leaning toward scrapping the serialized
    // execution feature anyway.
    let result = v8_try!(scope, raw_serde::serialize(scope, args.get(2)));
    let result_json: serde_json::Value = v8_try!(scope, from_v8(scope, args.get(2)));
    let events = get_event_state!(scope);

    if events.next_event < events.saved_results.len() {
        throw_error(scope, "Non-determinstic execution: Tried to save a new result but there were pending saved results");
        return;
    }

    let new_wall_time = Utc::now();
    events.wall_time = new_wall_time;
    events.new_results.push(SerializedEvent {
        wall_time: new_wall_time,
        fn_name,
        args_json,
        result,
        result_json,
    });
}

/// Get the next result, if any. This is called with the arguments object,
/// which is checked against the saved events object to ensure that the functions
/// are being called with the same sequence and arguments as what was saved.
/// From Javascript: getResult(exitIfUnsaved, fnName, fnArguments);
///
/// If there are no more saved results, the behavior depends on the value of the exitIfUnsaved
/// argument. If true, execution stops immediately so that the event can be handled externally.
/// If exitIfUnsaved is false, getResult returns a Symbol defined specially for this purpose
/// and accessible from JS at `window.ErgoSerialize.noNewResults`. This prevents confusion
/// between legitimate return values of `undefined`, `null`, or other values that are
/// normally used to represent a "no data" state.
fn get_result(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() != 3 {
        throw_error(
            scope,
            "Usage: getResult(exitIfUnsaved, functionName, functionArgs)",
        );
        return;
    }

    let exit_if_unsaved: bool = v8_try!(
        scope,
        from_v8(scope, args.get(0)),
        "First arguments should be exitIfUnsaved: boolean"
    );
    let fn_name: String = v8_try!(
        scope,
        from_v8(scope, args.get(1)),
        "Second argument must be the name of the function"
    );

    let fn_args = args.get(2);
    let args: Vec<serde_json::Value> = v8_try!(scope, from_v8(scope, fn_args));
    let events = get_event_state!(scope);

    match (events.saved_results.get(events.next_event), exit_if_unsaved) {
        (Some(event), _) => {
            // Make sure the function name and the arguments match. If not,
            // then the JS code is not running deterministically.
            if fn_name != event.fn_name || args != event.args_json {
                let msg = format!(
                    "Non-deterministic execution: expected function '{}' and matching arguments",
                    event.fn_name
                );
                throw_error(scope, &msg);
                return;
            }

            // Move wall time up to the saved time for this event.
            events.wall_time = event.wall_time;
            events.next_event += 1;
            // events holds a mutable borrow on scope, so clone the result to allow
            // events to implicitly drop and deserialize to take the mutable borrow. Not ideal but it works.
            let result = event.result.clone();
            let obj = v8_try!(scope, deserialize(scope, &result));
            rv.set(obj);
        }
        (None, true) => {
            // Save the requested event, and exit execution.
            events.pending_event = Some(PendingEvent {
                name: fn_name,
                args,
                result: None,
            });

            exit(scope);
        }
        (None, false) => {
            // Return to the caller that we didn't find anything.
            let symbol = events.no_new_results_symbol.clone();
            rv.set(v8::Local::new(scope, symbol).into());
        }
    };
}

/// Get the serialized wall time.
fn wall_time_accessor(
    scope: &mut v8::HandleScope,
    _name: v8::Local<v8::Name>,
    _args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let events = get_event_state!(scope);
    let time_ms = events.wall_time.timestamp_millis();
    let time_v8 = v8_try!(scope, to_v8(scope, &time_ms));
    rv.set(time_v8);
}

fn throw_error(scope: &mut v8::HandleScope, err: &str) {
    let msg = v8::String::new(scope, err).unwrap();
    let exc = Exception::error(scope, msg);
    scope.throw_exception(exc);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{net_extensions, PrintConsole, Runtime, RuntimeOptions};

    #[test]
    fn empty_event_list() {
        let script = r##"
            let result = globalThis.ErgoSerialize.getResult(false, 'fn', ['some args']);
            result === globalThis.ErgoSerialize.noNewResults
        "##;

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            console: Some(Box::new(PrintConsole::new(crate::ConsoleLevel::Info))),
            ..Default::default()
        });

        let result: bool = runtime
            .run_expression("test", script)
            .expect("Execution succeeded");

        assert!(result, "result was noNewResults symbol");
    }

    #[tokio::test]
    async fn save_and_retrieve_events() {
        let save_script = r##"
            globalThis.ErgoSerialize.saveResult('fn_name', [5, 6], "a result");
            globalThis.ErgoSerialize.saveResult('another_name', [{ a: 5, b: 6 }], { answer: 'yes' });
            "##;

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });

        let state = runtime
            .run_serialized("save script", save_script)
            .await
            .expect("Execution suceeded")
            .expect("serialized state was empty");

        assert_eq!(state.events.len(), 2);
        assert_eq!(state.events[0].fn_name, "fn_name", "first event name");
        assert_eq!(state.events[1].fn_name, "another_name", "second event name");

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(state),
            ..Default::default()
        });

        let second_script = r##"
            let firstResult = globalThis.ErgoSerialize.getResult(true, 'fn_name', [5, 6]);
            if(firstResult !== 'a result') {
                throw new Error(`Expected first result to be 'a result' but saw ${JSON.stringify(firstResult)}`);
            }

            let secondResult = globalThis.ErgoSerialize.getResult(true, 'another_name', [{ a: 5, b: 6 }]);
            let saw = JSON.stringify(secondResult);
            if(saw !== `{"answer":"yes"}`) {
                throw new Error(`Expected second result to be { answer: yes } but saw ${saw}`);
            }

            let newResult = globalThis.ErgoSerialize.getResult(false, 'last_fn', [1, 2, 3]);
            if(newResult !== globalThis.ErgoSerialize.noNewResults) {
                throw new Error(`Expected new result to be noNewResults symbol but saw ${newResult}`);
            }

            globalThis.ErgoSerialize.saveResult('last_fn', [1, 2, 3], undefined);
            "##;

        let new_state = runtime
            .run_serialized("second script", second_script)
            .await
            .expect("Running second script")
            .expect("serialized state was empty");

        assert_eq!(new_state.events.len(), 3);
        assert_eq!(new_state.events[0].fn_name, "fn_name", "first event name");
        assert_eq!(
            new_state.events[1].fn_name, "another_name",
            "second event name"
        );
        assert_eq!(new_state.events[2].fn_name, "last_fn", "third");
    }

    #[tokio::test]
    async fn catch_fn_name_mismatch() {
        let save_script =
            r##"globalThis.ErgoSerialize.saveResult('fn_name', [1, 2], "a result");"##;
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });
        let state = runtime
            .run_serialized("save script", save_script)
            .await
            .expect("save script")
            .expect("serialized state was empty");

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(state),
            ..Default::default()
        });

        let bad_get_script = r##"globalThis.ErgoSerialize.getResult(false, 'bad_name', [1, 2]);"##;
        let err = runtime
            .run_serialized("bad get script", bad_get_script)
            .await
            .expect_err("Expected error")
            .to_string();

        println!("{}", err);
        assert!(
            err.contains("Non-deterministic"),
            "Error should be a non-determinstic execution error",
        );
    }

    #[tokio::test]
    async fn catch_fn_args_mismatch() {
        let save_script =
            r##"globalThis.ErgoSerialize.saveResult('fn_name', [1, 2], "a result");"##;
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });
        let state = runtime
            .run_serialized("save script", save_script)
            .await
            .expect("save script")
            .expect("serialized state was empty");

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(state),
            ..Default::default()
        });

        let bad_get_script = r##"globalThis.ErgoSerialize.getResult(false, 'fn_name', [1, 3]);"##;
        let err = runtime
            .execute_script("bad get script", bad_get_script)
            .expect_err("Expected error")
            .to_string();

        println!("{}", err);
        assert!(
            err.contains("Non-deterministic"),
            "Error should be a non-determinstic execution error",
        );
    }

    #[tokio::test]
    async fn catch_save_with_pending_results() {
        let first_script = r##"
            ErgoSerialize.saveResult('a', [1, 2], 5);
            ErgoSerialize.saveResult('b', [3, 4], 6);
            "##;

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });

        let state = runtime
            .run_serialized("first", first_script)
            .await
            .expect("first script")
            .expect("serialized state was empty");

        let second_script = r##"
            ErgoSerialize.getResult(false, 'a', [1, 2]);
            ErgoSerialize.saveResult('c', [10], 11);
            "##;
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(state),
            ..Default::default()
        });

        let err = runtime
            .run_serialized("second", second_script)
            .await
            .expect_err("second script should fail")
            .to_string();
        println!("{}", err);
        assert!(
            err.contains("Tried to save a new result but there were pending saved results"),
            "Error should be about pending saved results"
        );
    }

    #[tokio::test]
    async fn wall_time() {
        let script = r##"
            const fn = ErgoSerialize.wrapSyncFunction(() => 5);
            let firstDateNum = Date.now();
            let firstDate = new Date();
            if(firstDate.valueOf() !== firstDateNum) {
                throw new Error(`First date was ${firstDate.valueOf()} but Date.now was ${firstDateNum}`);
            }

            let setDate = new Date(2200, 00, 01);
            if(setDate.getFullYear() !== 2200) {
                throw new Error(`Expected explicit year to be set but saw ${setDate.toString()}`);
            }

            // Calling into the serialize framework updates wall time.
            fn();

            let secondDateNum = Date.now();

            ({
                firstDateNum,
                secondDateNum
            })

        "##;
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Result {
            first_date_num: i64,
            second_date_num: i64,
        }

        let result: Result = runtime
            .run_expression("script 1", script)
            .expect("First script run");
        let state = runtime
            .take_serialize_state()
            .expect("take_serialize_state");
        println!("{:?}", result);
        assert_eq!(
            result.first_date_num,
            state.start_time.timestamp_millis(),
            "first time matches start time"
        );
        assert_eq!(
            result.second_date_num,
            state.events[0].wall_time.timestamp_millis(),
            "second time matches saved event time"
        );

        // In case the above all runs super fast, sleep a bit to make sure the second run actually
        // happens at a later time in ms.
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        let second_start_time = Utc::now();

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(state),
            ..Default::default()
        });

        let second_result: Result = runtime
            .run_expression("script 1", script)
            .expect("First script run");
        println!("{:?}", second_result);
        let state = runtime
            .take_serialize_state()
            .expect("take_serialize_state");
        assert_eq!(
            second_result.first_date_num,
            state.start_time.timestamp_millis(),
            "first time matches start time"
        );
        assert_eq!(
            second_result.second_date_num,
            state.events[0].wall_time.timestamp_millis(),
            "second time matches saved event time"
        );
        assert!(second_result.first_date_num < second_start_time.timestamp_millis());
    }

    #[test]
    fn exit() {
        let exit_script = r##"
            globalThis.x = 5;
            globalThis.ErgoSerialize.exit();
            globalThis.x = 6;
            "##;
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });

        let err = runtime
            .execute_script("exit_script", exit_script)
            .expect_err("Script terminates early")
            .to_string();

        println!("{}", err);
        assert!(
            err.ends_with("execution terminated"),
            "Error should be from the termination"
        );

        let x: usize = runtime
            .get_global_value("x")
            .expect("value x deserialized")
            .expect("value x was set");
        assert_eq!(
            x, 5,
            "Execution should stopped when x is 5 and before it is set to 6"
        );
    }

    #[tokio::test]
    async fn exit_if_unsaved() {
        let script = r##"
            globalThis.x = 5;
            globalThis.ErgoSerialize.getResult(true, 'abc', [1, { a: 5 } ]);
            globalThis.x = 6;
            "##;
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });

        let state = runtime
            .run_serialized("script", script)
            .await
            .expect("running script")
            .expect("serialied state was empty");

        let x: usize = runtime
            .get_global_value("x")
            .expect("value x deserialized")
            .expect("value x was set");
        assert_eq!(
            x, 5,
            "Execution should stopped when x is 5 and before it is set to 6"
        );

        let pending = state.pending.expect("Pending event should be present");
        assert_eq!(pending.name, "abc");
        assert_eq!(
            pending.args,
            vec![serde_json::json!(1), serde_json::json!({"a":5})]
        );
    }

    #[test]
    fn wrap_sync_function() {
        let script = r##"
            function abc() { return globalThis.x; }
            const fn = ErgoSerialize.wrapSyncFunction(abc);
            fn()
            "##;

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });

        runtime.set_global_value("x", &6).unwrap();
        let ret: usize = runtime
            .run_expression("script", script)
            .expect("executing script first run");

        assert_eq!(ret, 6, "first run returns 6");

        let state = runtime.take_serialize_state().expect("has state");
        assert!(state.events.len() == 1);
        assert_eq!(state.events[0].fn_name, "abc");

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(state),
            ..Default::default()
        });

        // Although we're setting x to 10, the call to fn() should return the saved value of 6
        // instead. This is very contrived but works for testing.
        runtime.set_global_value("x", &10).unwrap();
        let ret: usize = runtime
            .run_expression("script", script)
            .expect("executing script second run");
        assert_eq!(ret, 6, "second run returns saved value of 6");
    }

    #[test]
    fn wrap_sync_function_error() {
        let script = r##"
            function abc() {
                if(globalThis.x === 6) {
                    throw new Error('an error');
                } else {
                    return globalThis.x;
                }
            }
            const fn = ErgoSerialize.wrapSyncFunction(abc);

            globalThis.threw = false;
            try {
                fn();
            } catch(e) {
                globalThis.threw = true;
            }
            "##;

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });

        runtime.set_global_value("x", &6).unwrap();
        runtime
            .execute_script("script", script)
            .expect("executing script first run");

        let threw: bool = runtime.get_global_value("threw").unwrap().unwrap();
        assert_eq!(threw, true, "first run threw error");

        let state = runtime.take_serialize_state().expect("has state");
        assert!(state.events.len() == 1);
        assert_eq!(state.events[0].fn_name, "abc");

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(state),
            ..Default::default()
        });

        // Although we're setting x to 10, the call to fn() should return the saved value of 6
        // instead. This is very contrived but works for testing.
        runtime.set_global_value("x", &10).unwrap();
        runtime
            .execute_script("script", script)
            .expect("executing script second run");
        let threw: bool = runtime.get_global_value("threw").unwrap().unwrap();
        assert_eq!(threw, true, "second run threw saved error");
    }

    #[test]
    #[ignore]
    fn wrap_async_function() {}

    #[test]
    fn external_action() {
        let script = r##"
            const fn = ErgoSerialize.externalAction('abc');
            fn(1, 2, 3)
            "##;
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });

        let err = runtime
            .execute_script("script", script)
            .expect_err("script first run");

        println!("{:?}", err);
        let state = runtime
            .take_serialize_state()
            .expect("take_serialize_state");
        assert!(state.pending.is_some());
    }

    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn wrap_fetch() {
        // Fetch requires some extra wrapping logic to save the body, so test it separately.

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("a response"))
            .mount(&server)
            .await;

        let script = r##"
            (async function run() {
               let x = await globalThis.fetch(url);
               globalThis.result = await x.text();
            }());
            "##;

        let state = SerializedState::default();
        let mut runtime = Runtime::new(RuntimeOptions {
            extensions: crate::net_extensions(Some(state.random_seed)),
            serialized_state: Some(state),
            console: Some(Box::new(PrintConsole::new(crate::ConsoleLevel::Info))),
            ..Default::default()
        });

        runtime.set_global_value("url", &server.uri()).unwrap();
        let state = runtime
            .run_serialized("script", script)
            .await
            .expect("script run 1")
            .expect("serialized state was empty");

        let result: String = runtime.get_global_value("result").unwrap().unwrap();
        assert_eq!(result, "a response");

        assert_eq!(state.events.len(), 1, "state saved the event");
        println!("{:?}", state.events[0]);

        // And now run it again.
        let mut runtime = Runtime::new(RuntimeOptions {
            extensions: crate::net_extensions(Some(state.random_seed)),
            serialized_state: Some(state),
            console: Some(Box::new(PrintConsole::new(crate::ConsoleLevel::Info))),
            ..Default::default()
        });

        runtime.set_global_value("url", &server.uri()).unwrap();

        // Shutdown the mock server to make sure that the fetch is coming from the
        // saved state and not from doing an actual fetch.
        server.reset().await;
        drop(server);

        runtime
            .run_serialized("script-run-2", script)
            .await
            .expect("script run 2");
        let result: String = runtime.get_global_value("result").unwrap().unwrap();
        assert_eq!(result, "a response");
    }

    #[tokio::test]
    async fn random() {
        let script = r##"
            let x = Math.random();
            let y = Math.random();
            [x, y]
        "##;

        let state = SerializedState::default();
        println!("Random seed: {}", state.random_seed);
        let mut runtime = Runtime::new(RuntimeOptions {
            extensions: net_extensions(Some(state.random_seed)),
            serialized_state: Some(state),
            ..Default::default()
        });

        let first_values: Vec<f64> = runtime
            .run_expression("script", script)
            .expect("first script run");
        println!("First values {:?}", first_values);
        assert_ne!(
            first_values[0], first_values[1],
            "successive Math.random() calls in a script are different"
        );

        let state = runtime
            .take_serialize_state()
            .expect("take_serialize_state");
        let mut runtime = Runtime::new(RuntimeOptions {
            extensions: net_extensions(Some(state.random_seed)),
            serialized_state: Some(state),
            ..Default::default()
        });

        let second_values: Vec<f64> = runtime
            .run_expression("script", script)
            .expect("second script run");

        assert_eq!(
            first_values[0], second_values[0],
            "First value is same across successive runs"
        );
        assert_eq!(
            first_values[1], second_values[1],
            "Second value is same across successive runs"
        );
    }
}
