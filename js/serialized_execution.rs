use chrono::{DateTime, Utc};
use rusty_v8 as v8;
use serde::{Deserialize, Serialize};
use serde_v8::to_v8;
use v8::{Exception, MapFnTo};

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

fn save_result(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    todo!();
}

/// Get the next result, if any.
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
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let events = match scope.get_slot::<EventTracker>() {
        Some(e) => e,
        None => {
            let msg = v8::String::new(scope, "Serialized execution not enabled").unwrap();
            let exc = Exception::error(scope, msg);
            scope.throw_exception(exc);
            return;
        }
    };

    let time_ms = events.wall_time.timestamp_millis();
    let time_v8 = to_v8(scope, &time_ms).unwrap();
    rv.set(time_v8);
}
