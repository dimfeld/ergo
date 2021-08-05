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
    random_seed: u64,
    start_time: chrono::DateTime<Utc>,
    events: Vec<SerializedEvent>,
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

/// Exit the script early.
fn exit(scope: &mut v8::HandleScope, _args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    scope.terminate_execution();
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

    let new_wall_time = Utc::now();
    events.wall_time = new_wall_time;
    events.saved_results.push(SerializedEvent {
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
                // the saved arguments and put them on the error somewhere.
                let msg = format!(
                    "Deterministic execution violation, expected function {}",
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
    #[ignore]
    fn save_and_retrieve_events() {}

    #[test]
    #[ignore]
    fn catch_fn_name_mismatch() {}

    #[test]
    #[ignore]
    fn catch_fn_args_mismatch() {}

    #[test]
    #[ignore]
    fn wall_time() {}

    #[test]
    #[ignore]
    fn exit() {}
}
