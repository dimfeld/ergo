use std::fmt::Display;

use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateFieldFormat {
    String,
    StringArray,
    Integer,
    Float,
    Object,
    Choice {
        choices: Vec<String>,
        max: Option<usize>,
    },
}

impl TemplateFieldFormat {
    fn validate(
        &self,
        field_name: &str,
        value: &serde_json::Value,
    ) -> Result<(), TemplateValidationFailure> {
        let actual_type = match value {
            serde_json::Value::String(_) => TemplateFieldFormat::String,
            serde_json::Value::Array(_) => TemplateFieldFormat::StringArray,
            serde_json::Value::Number(n) => {
                if n.is_i64() {
                    TemplateFieldFormat::Integer
                } else {
                    TemplateFieldFormat::Float
                }
            }
            _ => TemplateFieldFormat::Object,
        };

        if self == &actual_type {
            return Ok(());
        }

        match (self, actual_type) {
            (Self::Choice { choices, max }, Self::String) => {
                todo!();
            }
            (Self::Choice { choices, max }, Self::StringArray) => {
                todo!();
            }
            (Self::Float, Self::Integer) => Ok(()),
            (_, actual) => Err(TemplateValidationFailure::Invalid {
                name: field_name.to_string(),
                expected: self.clone(),
                actual,
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateField {
    pub name: String,
    pub format: TemplateFieldFormat,
    pub optional: bool,
    pub description: Option<String>,
}

pub type TemplateFields = Vec<TemplateField>;

#[derive(Debug)]
pub enum TemplateValidationFailure {
    Required(String),
    Invalid {
        name: String,
        expected: TemplateFieldFormat,
        actual: TemplateFieldFormat,
    },
}

impl Display for TemplateValidationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateValidationFailure::Required(name) => write!(f, "Field {} is required", name),
            TemplateValidationFailure::Invalid {
                name,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Field {} expected type {:?}, saw {:?}",
                    name, expected, actual
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct TemplateValidationError {
    object: &'static str,
    id: i64,
    fields: Vec<TemplateValidationFailure>,
}

impl std::error::Error for TemplateValidationError {}

impl Display for TemplateValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Template validation failure for {} {}",
            self.object, self.id
        )?;
        for field in &self.fields {
            writeln!(f, "\t{}", field)?;
        }
        Ok(())
    }
}

// pub trait MapGetter {
//     fn get(&self, key: &String) -> Option<&serde_json::Value>;
// }
//
// impl MapGetter for std::collections::HashMap<String, serde_json::Value> {
//     fn get(&self, key: &String) -> Option<&serde_json::Value> {
//         std::collections::HashMap::get(self, key)
//     }
// }
//
// impl MapGetter for serde_json::Map<String, serde_json::Value> {
//     fn get(&self, key: &String) -> Option<&serde_json::Value> {
//         serde_json::Map::get(self, key)
//     }
// }

pub fn validate(
    object: &'static str,
    id: i64,
    fields: &TemplateFields,
    values: &FxHashMap<String, serde_json::Value>,
) -> Result<(), TemplateValidationError> {
    let errors = fields
        .iter()
        .filter_map(|field| match (values.get(&field.name), field.optional) {
            (Some(v), _) => Some(field.format.validate(&field.name, &v)),
            (None, true) => None,
            (None, false) => Some(Err(TemplateValidationFailure::Required(field.name.clone()))),
        })
        .filter_map(|e| match e {
            Ok(_) => None,
            Err(e) => Some(e),
        })
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(TemplateValidationError {
            object,
            id,
            fields: errors,
        })
    }
}
