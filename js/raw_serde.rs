//! This code is inspired by the serialize/deserialize functions in deno/core/bindings.rs.
use thiserror::Error;
use v8::{Local, Value, ValueDeserializerHelper, ValueSerializerHelper};

#[derive(Debug, Error)]
pub enum RawSerdeError {
    #[error("Failed to serialize object")]
    Serialize,
    #[error("Failed to deserialize object")]
    Deserialize,
}

struct SerializeDeserialize {}

impl v8::ValueSerializerImpl for SerializeDeserialize {
    #[allow(unused_variables)]
    fn throw_data_clone_error<'s>(
        &mut self,
        scope: &mut v8::HandleScope<'s>,
        message: v8::Local<'s, v8::String>,
    ) {
        let error = v8::Exception::error(scope, message);
        scope.throw_exception(error);
    }
}

impl v8::ValueDeserializerImpl for SerializeDeserialize {}

/// Serialize a V8 object into a byte vector for storage and later retrieval.
pub fn serialize(
    scope: &mut v8::HandleScope,
    value: Local<Value>,
) -> Result<Vec<u8>, RawSerdeError> {
    let serialize_deserialize = Box::new(SerializeDeserialize {});
    let mut value_serializer = v8::ValueSerializer::new(scope, serialize_deserialize);
    value_serializer.write_header();
    match value_serializer.write_value(scope.get_current_context(), value) {
        Some(true) => Ok(value_serializer.release()),
        _ => Err(RawSerdeError::Serialize),
    }
}

/// Deserialize a byte vector into the objects it represents.
pub fn deserialize<'a>(
    scope: &'a mut v8::HandleScope,
    data: &[u8],
) -> Result<Local<'a, Value>, RawSerdeError> {
    let serialize_deserialize = Box::new(SerializeDeserialize {});
    let mut value_deserializer = v8::ValueDeserializer::new(scope, serialize_deserialize, data);

    let parsed = value_deserializer
        .read_header(scope.get_current_context())
        .unwrap_or_default();
    if !parsed {
        return Err(RawSerdeError::Deserialize);
    }

    value_deserializer
        .read_value(scope.get_current_context())
        .ok_or(RawSerdeError::Deserialize)
}
