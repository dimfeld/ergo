use std::fmt::Display;

use fxhash::FxHashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use thiserror::Error;

lazy_static! {
    static ref HANDLEBARS: handlebars::Handlebars<'static> = handlebars::Handlebars::new();
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("{0}")]
    Validation(#[from] TemplateValidationError),
    #[error("{0}")]
    Render(#[from] handlebars::TemplateRenderError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateFieldFormat {
    String,
    StringArray,
    Integer,
    Float,
    Boolean,
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
            (Self::Float, Self::String) => todo!(), // Verify can be coerced to a float
            (Self::Integer, Self::String) => todo!(), // Verify can be coerced to an integer
            (Self::Boolean, Self::String) => todo!(), // Verify can be coerced to a boolean
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
    pub format: TemplateFieldFormat,
    pub optional: bool,
    pub description: Option<String>,
}

pub type TemplateFields = FxHashMap<String, TemplateField>;

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
    id: String,
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
    id: impl ToString,
    fields: &TemplateFields,
    values: &FxHashMap<String, serde_json::Value>,
) -> Result<(), TemplateError> {
    let errors = fields
        .iter()
        .filter_map(|(name, field)| match (values.get(name), field.optional) {
            (Some(v), _) => Some(field.format.validate(name, &v)),
            (None, true) => None,
            (None, false) => Some(Err(TemplateValidationFailure::Required(name.to_string()))),
        })
        .filter_map(|e| match e {
            Ok(_) => None,
            Err(e) => Some(e),
        })
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(TemplateError::Validation(TemplateValidationError {
            object,
            id: id.to_string(),
            fields: errors,
        }))
    }
}

pub fn apply_field(
    template: &serde_json::Value,
    values: &FxHashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, handlebars::TemplateRenderError> {
    let result = match template {
        serde_json::Value::String(template) => {
            let rendered = HANDLEBARS.render_template(template, values)?;

            let trimmed = rendered.trim();
            let result = if trimmed.len() == rendered.len() {
                rendered
            } else {
                trimmed.to_string()
            };

            serde_json::Value::String(result)
        }
        serde_json::Value::Array(template) => {
            let output_array = template
                .iter()
                .map(|t| apply_field(t, values))
                .collect::<Result<Vec<_>, _>>()?;
            serde_json::Value::Array(output_array)
        }
        serde_json::Value::Object(o) => {
            let output_object = o
                .iter()
                .map(|(k, v)| {
                    let mapped_value = apply_field(v, values)?;
                    Ok::<_, handlebars::TemplateRenderError>((k.clone(), mapped_value))
                })
                .collect::<Result<serde_json::Map<String, _>, _>>()?;
            serde_json::Value::Object(output_object)
        }
        s => s.clone(),
    };

    Ok(result)
}

pub fn validate_and_apply<'a>(
    object: &'static str,
    id: i64,
    fields: &TemplateFields,
    template: &'a Vec<(String, serde_json::Value)>,
    values: &FxHashMap<String, serde_json::Value>,
) -> Result<FxHashMap<String, serde_json::Value>, TemplateError> {
    validate(object, id, fields, values)?;
    let output = template
        .iter()
        .map(|(name, field_template)| {
            apply_field(field_template, values).map(|rendered| (name.to_string(), rendered))
        })
        .collect::<Result<FxHashMap<_, _>, _>>()?;

    Ok(output)
}
