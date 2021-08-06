use chrono::{DateTime, Utc};
use rusty_v8 as v8;
use serde::{Deserialize, Serialize};
use serde_v8::{from_v8, to_v8};
use v8::{Exception, MapFnTo};

use crate::raw_serde::{self, deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializedEvent {
    /// The wall time when the event completed.
    wall_time: chrono::DateTime<Utc>,
    /// Serialized version of the arguments used to generate this result, which will
    /// be compared on retrieval to ensure that the script is running deterministically.
    args: Vec<u8>,
    fn_name: String,
    result: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedState {
    pub random_seed: u64,
    pub start_time: chrono::DateTime<Utc>,
    pub events: Vec<SerializedEvent>,
}

impl Default for SerializedState {
    fn default() -> Self {
        SerializedState {
            random_seed: rand::random(),
            start_time: Utc::now(),
            events: Vec::new(),
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct EventTracker {
    wall_time: DateTime<Utc>,
    saved_results: Vec<SerializedEvent>,
    next_event: usize,
    new_results: Vec<SerializedEvent>,
    no_new_results_symbol: v8::Global<v8::Symbol>,

    start_time: DateTime<Utc>,
    random_seed: u64,
}

pub fn install(runtime: &mut deno_core::JsRuntime, history: SerializedState) {
    let scope = &mut runtime.handle_scope();

    let jskey = v8::String::new(scope, "ErgoSerialize").unwrap();
    let ser_obj = v8::Object::new(scope);

    set_func(scope, ser_obj, "saveResult", save_result);
    set_func(scope, ser_obj, "getResult", get_result);
    set_func(scope, ser_obj, "exit", exit);

    let no_new_results_symbol_name = v8::String::new(scope, "ErgoSerialize noNewResults").unwrap();
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

        random_seed: history.random_seed,
        start_time: history.start_time,
    };
    scope.set_slot(tracker);
}

/// Extract the serialized state from the runtime. This clears the saved state to avoid cloning,
/// so should only be done once this runtime is finished.
pub fn take_serialize_state(runtime: &mut crate::Runtime) -> Option<SerializedState> {
    let isolate = runtime.runtime.v8_isolate();
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
                random_seed: 0,
                start_time: now,
            };

            Some(std::mem::replace(e, replacement).into())
        }
        None => None,
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
fn exit(scope: &mut v8::HandleScope, _args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
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

    let fn_args = v8_try!(scope, raw_serde::serialize(scope, args.get(1)));
    let result = v8_try!(scope, raw_serde::serialize(scope, args.get(2)));
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
        args: fn_args,
        result,
    });
}

/// Get the next result, if any. This is called with the arguments object,
/// which is checked against the saved events object to ensure that the functions
/// are being called with the same sequence and arguments as what was saved.
/// From Javascript: getResult(fnName, fnArguments);
///
/// If there are no more saved results, this returns a Symbol defined specially for this purpose
/// and accessible from JS at `window.ErgoSerialize.noNewResults`. This prevents confusion
/// between legitimate return values of `undefined`, `null`, or other values that are
/// normally used to represent a "no data" state.
fn get_result(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() != 2 {
        throw_error(
            scope,
            "Arguments must be the function name and its arguments",
        );
        return;
    }

    let fn_name: String = v8_try!(
        scope,
        from_v8(scope, args.get(0)),
        "First argument must be the name of the function"
    );
    let fn_args = v8_try!(scope, raw_serde::serialize(scope, args.get(1)));
    let events = get_event_state!(scope);

    match events.saved_results.get(events.next_event) {
        Some(event) => {
            // Make sure the function name and the arguments match. If not,
            // then the JS code is not running deterministically.
            if fn_name != event.fn_name || fn_args != event.args {
                // TODO More descriptive error that details the differences. Probably deserialize
                // the saved arguments and put them on the error somewhere. Have to figure out the
                // mutable borrow issues with deserialze results first though.
                let msg = format!(
                    "Non-deterministic execution: expected function '{}' and matching arguments",
                    event.fn_name
                );
                throw_error(scope, &msg);
                return;
            }

            events.next_event += 1;
            // events holds a mutable borrow on scope, so clone the result and drop events to allow
            // deserialize to take the mutable borrow. Not ideal but it works.
            let result = event.result.clone();
            drop(events);

            let obj = v8_try!(scope, deserialize(scope, &result));
            rv.set(obj);
        }
        None => {
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
    use crate::{Runtime, RuntimeOptions};

    fn get_event_tracker<'a>(runtime: &'a mut crate::Runtime) -> &'a mut EventTracker {
        runtime.v8_isolate().get_slot_mut::<EventTracker>().unwrap()
    }

    #[test]
    fn empty_event_list() {
        let script = r##"
            let result = globalThis.ErgoSerialize.getResult('fn', ['some args']);
            result === globalThis.ErgoSerialize.noNewResults
        "##;

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });

        let result: bool = runtime
            .run_expression("test", script)
            .expect("Execution succeeded");

        assert!(result, "result was noNewResults symbol");
    }

    #[test]
    fn save_and_retrieve_events() {
        let save_script = r##"
            globalThis.ErgoSerialize.saveResult('fn_name', [5, 6], "a result");
            globalThis.ErgoSerialize.saveResult('another_name', { a: 5, b: 6 }, { answer: 'yes' });
            "##;

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });

        runtime
            .execute_script("save script", save_script)
            .expect("Execution suceeded");

        let state = take_serialize_state(&mut runtime).expect("take_serialize_state first call");

        assert_eq!(state.events.len(), 2);
        assert_eq!(state.events[0].fn_name, "fn_name", "first event name");
        assert_eq!(state.events[1].fn_name, "another_name", "second event name");

        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(state),
            ..Default::default()
        });

        let second_script = r##"
            let firstResult = globalThis.ErgoSerialize.getResult('fn_name', [5, 6]);
            if(firstResult !== 'a result') {
                throw new Error(`Expected first result to be 'a result' but saw ${JSON.stringify(firstResult)}`);
            }

            let secondResult = globalThis.ErgoSerialize.getResult('another_name', { a: 5, b: 6 });
            let saw = JSON.stringify(secondResult);
            if(saw !== `{"answer":"yes"}`) {
                throw new Error(`Expected second result to be { answer: yes } but saw ${saw}`);
            }

            let newResult = globalThis.ErgoSerialize.getResult('last_fn', [1, 2, 3]);
            if(newResult !== globalThis.ErgoSerialize.noNewResults) {
                throw new Error(`Expected new result to be noNewResult symbol but saw ${newResult}`);
            }

            globalThis.ErgoSerialize.saveResult('last_fn', [1, 2, 3], undefined);
            "##;

        runtime
            .execute_script("second script", second_script)
            .expect("Running second script");

        let new_state =
            take_serialize_state(&mut runtime).expect("take_serialize_state second call");

        assert_eq!(new_state.events.len(), 3);
        assert_eq!(new_state.events[0].fn_name, "fn_name", "first event name");
        assert_eq!(
            new_state.events[1].fn_name, "another_name",
            "second event name"
        );
        assert_eq!(new_state.events[2].fn_name, "last_fn", "third");
    }

    #[test]
    fn catch_fn_name_mismatch() {
        let save_script =
            r##"globalThis.ErgoSerialize.saveResult('fn_name', [1, 2], "a result");"##;
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });
        runtime
            .execute_script("save script", save_script)
            .expect("save script");

        let state = take_serialize_state(&mut runtime).expect("take_serialize_state");
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(state),
            ..Default::default()
        });

        let bad_get_script = r##"globalThis.ErgoSerialize.getResult('bad_name', [1, 2]);"##;
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

    #[test]
    fn catch_fn_args_mismatch() {
        let save_script =
            r##"globalThis.ErgoSerialize.saveResult('fn_name', [1, 2], "a result");"##;
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(SerializedState::default()),
            ..Default::default()
        });
        runtime
            .execute_script("save script", save_script)
            .expect("save script");

        let state = take_serialize_state(&mut runtime).expect("take_serialize_state");
        let mut runtime = Runtime::new(RuntimeOptions {
            serialized_state: Some(state),
            ..Default::default()
        });

        let bad_get_script = r##"globalThis.ErgoSerialize.getResult('fn_name', [1, 3]);"##;
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

    #[test]
    #[ignore]
    fn catch_save_with_pending_results() {}

    #[test]
    #[ignore]
    fn wall_time() {}

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
            err.contains("execution terminated"),
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
}
