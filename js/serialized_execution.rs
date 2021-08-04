use chrono::{DateTime, Utc};
use rusty_v8 as v8;
use serde::{Deserialize, Serialize};
use serde_v8::{from_v8, to_v8};
use v8::{Exception, MapFnTo};

use crate::raw_serde;

lazy_static::lazy_static! {
    pub static ref EXTERNAL_REFERENCES: v8::ExternalReferences = v8::ExternalReferences::new(&[
        v8::ExternalReference {
            function: save_result.map_fn_to()
        },
        v8::ExternalReference {
            function: get_result.map_fn_to()
        },
    ]);
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializedEvent {
    /// The wall time when the event completes.
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

#[derive(Debug, Clone)]
pub struct EventTracker {
    wall_time: DateTime<Utc>,
    saved_results: Vec<SerializedEvent>,
    next_event: usize,
    new_results: Vec<SerializedEvent>,
}

impl Default for EventTracker {
    fn default() -> Self {
        EventTracker {
            wall_time: Utc::now(),
            saved_results: Vec::new(),
            next_event: 0,
            new_results: Vec::new(),
        }
    }
}

pub fn install(runtime: &mut crate::Runtime, history: SerializedState) -> () {
    let tracker = EventTracker {
        saved_results: history.events,
        wall_time: history.start_time,
        next_event: 0,
        new_results: Vec::new(),
    };

    let scope = &mut runtime.runtime.handle_scope();
    scope.set_slot(tracker);

    let jskey = v8::String::new(scope, "ErgoSerialize").unwrap();
    let ser_obj = v8::Object::new(scope);

    set_func(scope, ser_obj, "saveResult", save_result);
    set_func(scope, ser_obj, "getResult", get_result);
    set_func(scope, ser_obj, "wallTime", wall_time);
    set_func(scope, ser_obj, "exit", exit);

    let global = scope.get_current_context().global(scope);
    global.set(scope, jskey.into(), ser_obj.into());
}

/// Extract the serialized state from the runtime. This clears the saved state to avoid cloning,
/// so should only be done once this runtime is finished.
pub fn take_serialize_state(runtime: &mut crate::Runtime) -> Option<EventTracker> {
    let isolate = runtime.runtime.v8_isolate();
    let events = isolate.get_slot_mut::<EventTracker>();

    match events {
        Some(e) => Some(std::mem::take(e)),
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

macro_rules! convert_arg {
    ($scope: expr, $value: expr, $err: expr) => {
        match from_v8($scope, $value) {
            Ok(value) => value,
            Err(_) => {
                throw_error($scope, $err);
                return;
            }
        }
    };
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

fn save_result(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let fn_name: String = convert_arg!(
        scope,
        args.get(0),
        "First argument must be the name of the function"
    );

    let fn_args = raw_serde::serialize(scope, args.get(1)).unwrap();
    let result = raw_serde::serialize(scope, args.get(2)).unwrap();
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
/// From Javascript: getResult(argumentsObject);
fn get_result(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    todo!();
}

/// Get the serialized wall time.
fn wall_time(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let events = get_event_state!(scope);
    let time_ms = events.wall_time.timestamp_millis();
    let time_v8 = to_v8(scope, &time_ms).unwrap();
    rv.set(time_v8);
}

fn throw_error(scope: &mut v8::HandleScope, err: &'static str) {
    let msg = v8::String::new(scope, err).unwrap();
    let exc = Exception::error(scope, msg);
    scope.throw_exception(exc);
}
