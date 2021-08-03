use deno_core::{
    error::{null_opbuf, AnyError},
    include_js_files, op_async, op_sync, Extension, OpState, ResourceId, ZeroCopyBuf,
};
use rusty_v8 as v8;
use serde::{Deserialize, Serialize};
use v8::MapFnTo;

pub type SerializedEvent = Vec<u8>;

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

#[derive(Serialize, Deserialize)]
pub struct SerializedState {
    random_seed: u64,
    events: Vec<SerializedEvent>,
}

impl Default for SerializedState {
    fn default() -> Self {
        SerializedState {
            random_seed: rand::random(),
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct EventTracker {
    saved_results: Vec<SerializedEvent>,
    next_event: usize,
    new_results: Vec<SerializedEvent>,
}

pub fn install(runtime: &mut crate::Runtime, history: SerializedState) -> () {
    let tracker = EventTracker {
        saved_results: history.events,
        next_event: 0,
        new_results: Vec::new(),
    };
    let scope = &mut runtime.runtime.handle_scope();
    scope.set_slot(tracker);

    let jskey = v8::String::new(scope, "ErgoSerialize").unwrap();
    let ser_obj = v8::Object::new(scope);

    set_func(scope, ser_obj, "saveResult", save_result);
    set_func(scope, ser_obj, "getResult", get_result);

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

fn get_result(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    todo!();
}
