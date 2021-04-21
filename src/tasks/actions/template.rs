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
    fn validate(&self, value: &serde_json::Value) -> Result<(), TemplateValidationFailure> {
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
                expected: self.clone(),
                actual,
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateField {
    pub format: TemplateFieldFormat,
    pub optional: bool,
    pub description: Option<String>,
}

pub type TemplateFields = FxHashMap<String, TemplateField>;

#[derive(Debug)]
pub enum TemplateValidationFailure {
    Required,
    Invalid {
        expected: TemplateFieldFormat,
        actual: TemplateFieldFormat,
    },
}

impl Display for TemplateValidationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateValidationFailure::Required => write!(f, "Field is required"),
            TemplateValidationFailure::Invalid { expected, actual } => {
                write!(f, "Expected type {:?}, saw {:?}", expected, actual)
            }
        }
    }
}

#[derive(Debug)]
pub struct TemplateValidationError {
    object: &'static str,
    id: i64,
    fields: Vec<(String, TemplateValidationFailure)>,
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
            writeln!(f, "\t{}: {}", field.0, field.1)?;
        }
        Ok(())
    }
}

pub fn validate(
    fields: &TemplateFields,
    values: impl IntoIterator<Item = (String, serde_json::Value)>,
) -> Result<(), TemplateValidationError> {
    todo!("Validate template");
}
