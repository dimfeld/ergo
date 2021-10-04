#[cfg(not(target_family = "wasm"))]
use sqlx::{postgres::PgTypeInfo, Database};
use std::{ops::Deref, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

use crate::new_uuid;

#[derive(Debug, Error)]
pub enum ObjectIdError {
    #[error("Invalid ID prefix, expected {0}")]
    InvalidPrefix(&'static str),

    #[error("Failed to decode object ID")]
    DecodeFailure,
}

/// A type that is internally stored as a UUID but externally as a
/// more accessible string with a prefix indicating its type.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId<const PREFIX: usize>(pub Uuid);

pub type TaskId = ObjectId<0>;
pub type OrgId = ObjectId<1>;
pub type RoleId = ObjectId<2>;
pub type UserId = ObjectId<3>;
pub type InputId = ObjectId<4>;
pub type ActionId = ObjectId<5>;
pub type InputCategoryId = ObjectId<6>;
pub type ActionCategoryId = ObjectId<7>;
pub type AccountId = ObjectId<8>;
pub type TaskTriggerId = ObjectId<9>;
pub type TaskTemplateId = ObjectId<10>;
pub type NotifyEndpointId = ObjectId<11>;
pub type NotifyListenerId = ObjectId<12>;

impl<const PREFIX: usize> ObjectId<PREFIX> {
    /// Once const generics supports strings, this can go away, but for now we
    /// do it this way.
    #[inline(always)]
    fn prefix() -> &'static str {
        match PREFIX {
            0 => "tsk",
            1 => "org",
            2 => "rl",
            3 => "usr",
            4 => "inp",
            5 => "act",
            6 => "icat",
            7 => "acat",
            8 => "acct",
            9 => "trg",
            10 => "tmpl",
            11 => "ne",
            12 => "nl",
            _ => "",
        }
    }

    pub fn new() -> Self {
        Self(new_uuid())
    }

    pub fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }

    pub fn into_inner(self) -> Uuid {
        self.0
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl<const PREFIX: usize> PartialEq<Uuid> for ObjectId<PREFIX> {
    fn eq(&self, other: &Uuid) -> bool {
        &self.0 == other
    }
}

impl<const PREFIX: usize> Deref for ObjectId<PREFIX> {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const PREFIX: usize> From<Uuid> for ObjectId<PREFIX> {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl<const PREFIX: usize> Into<Uuid> for ObjectId<PREFIX> {
    fn into(self) -> Uuid {
        self.0
    }
}

impl<const PREFIX: usize> std::fmt::Debug for ObjectId<PREFIX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjectId")
            .field(&self.to_string())
            .field(&self.0)
            .finish()
    }
}

impl<const PREFIX: usize> std::fmt::Display for ObjectId<PREFIX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::prefix())?;
        base64::display::Base64Display::with_config(self.0.as_bytes(), base64::URL_SAFE_NO_PAD)
            .fmt(f)
    }
}

pub fn decode_suffix(s: &str) -> Result<Uuid, ObjectIdError> {
    let bytes = base64::decode_config(s, base64::URL_SAFE_NO_PAD)
        .map_err(|_| ObjectIdError::DecodeFailure)?;
    Uuid::from_slice(&bytes).map_err(|_| ObjectIdError::DecodeFailure)
}

impl<const PREFIX: usize> FromStr for ObjectId<PREFIX> {
    type Err = ObjectIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expected_prefix = Self::prefix();
        if !s.starts_with(expected_prefix) {
            return Err(ObjectIdError::InvalidPrefix(expected_prefix));
        }

        decode_suffix(&s[expected_prefix.len()..]).map(|u| Self(u))
    }
}

/// Serialize into string form with the prefix
impl<const PREFIX: usize> serde::Serialize for ObjectId<PREFIX> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

struct ObjectIdVisitor<const PREFIX: usize>;

impl<'de, const PREFIX: usize> serde::de::Visitor<'de> for ObjectIdVisitor<PREFIX> {
    type Value = ObjectId<PREFIX>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an object ID starting with ")?;
        formatter.write_str(Self::Value::prefix())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match Self::Value::from_str(v) {
            Ok(id) => Ok(id),
            Err(e) => {
                // See if it's in UUID format instead of the encoded format. This mostly happens when
                // deserializing from a JSON object generated in Postgres with jsonb_build_object.
                Uuid::from_str(v)
                    .map(|u| ObjectId::<PREFIX>::from_uuid(u))
                    // Return the more descriptive original error instead of the UUID parsing error
                    .map_err(|_| e)
            }
        }
        .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(v), &self))
    }
}

/// Deserialize from string form with the prefix.
impl<'de, const PREFIX: usize> serde::Deserialize<'de> for ObjectId<PREFIX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ObjectIdVisitor)
    }
}

impl<const PREFIX: usize> schemars::JsonSchema for ObjectId<PREFIX> {
    fn schema_name() -> String {
        String::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(gen)
    }
}

/// Store and retrieve in Postgres as a raw UUID
#[cfg(not(target_family = "wasm"))]
impl<const PREFIX: usize> sqlx::Type<sqlx::Postgres> for ObjectId<PREFIX> {
    fn type_info() -> <sqlx::Postgres as Database>::TypeInfo {
        PgTypeInfo::with_name("uuid")
    }
}

#[cfg(not(target_family = "wasm"))]
impl<'q, const PREFIX: usize> sqlx::Encode<'q, sqlx::Postgres> for ObjectId<PREFIX> {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Postgres as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        self.0.encode_by_ref(buf)
    }
}

#[cfg(not(target_family = "wasm"))]
impl<'r, const PREFIX: usize> sqlx::Decode<'r, sqlx::Postgres> for ObjectId<PREFIX> {
    fn decode(
        value: <sqlx::Postgres as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let u = Uuid::decode(value)?;
        Ok(Self(u))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_from_str() {
        let id = TaskId::new();

        let s = id.to_string();
        let id2 = TaskId::from_str(&s).unwrap();
        assert_eq!(id, id2, "ID converts to string and back");
    }

    #[test]
    fn serde() {
        let id = TaskId::new();
        let json_str = serde_json::to_string(&id).unwrap();
        let id2: TaskId = serde_json::from_str(&json_str).unwrap();
        assert_eq!(id, id2, "Value serializes and deserializes to itself");
    }
}
