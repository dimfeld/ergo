use std::fmt::Display;

use fxhash::FxHashMap;
use handlebars::TemplateRenderError;
use lazy_static::lazy_static;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

lazy_static! {
    static ref HANDLEBARS: handlebars::Handlebars<'static> = {
        let mut h = handlebars::Handlebars::new();
        h.strict_mode();
        h.register_escape_fn(|s| s.to_string());
        h
    };
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("{0}")]
    Validation(#[from] TemplateValidationError),
    #[error("{0}")]
    Render(#[from] handlebars::TemplateRenderError),
}

#[derive(Clone, Debug, JsonSchema, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TemplateFieldFormat {
    String,
    StringArray,
    Integer,
    Float,
    Boolean,
    Object,
    Choice {
        choices: Vec<String>,
        min: Option<usize>,
        max: Option<usize>,
    },
}

impl TemplateFieldFormat {
    fn validate(
        &self,
        field_name: &str,
        value: &serde_json::Value,
    ) -> Result<(), TemplateValidationFailure> {
        let ok = match value {
            serde_json::Value::String(s) => {
                match self {
                    Self::String => true,
                    Self::Integer => s.parse::<u64>().is_ok(),
                    Self::Float => s.parse::<f64>().is_ok(),
                    Self::Boolean => s.parse::<bool>().is_ok(),
                    Self::Choice { choices, min, .. } => {
                        if min.map(|m| m > 1).unwrap_or(false) {
                            // Requires more than one argument
                            false
                        } else {
                            choices.iter().find(|&x| x == s).is_some()
                        }
                    }
                    _ => false,
                }
            }
            serde_json::Value::Array(a) => match self {
                Self::StringArray => true,
                Self::Choice { choices, min, max } => {
                    if min.map(|m| m > a.len()).unwrap_or(false)
                        || max.map(|m| m < a.len()).unwrap_or(false)
                    {
                        false
                    } else {
                        a.iter().all(|value| {
                            choices
                                .iter()
                                .find(|&c| value.as_str().map(|s| s == c).unwrap_or(false))
                                .is_some()
                        })
                    }
                }
                _ => false,
            },
            serde_json::Value::Bool(_) => match self {
                Self::String | Self::Boolean => true,
                _ => false,
            },
            serde_json::Value::Number(n) => match self {
                Self::String | Self::Float => true,
                Self::Integer => n.is_i64(),
                _ => false,
            },
            serde_json::Value::Object(_) => *self == TemplateFieldFormat::Object,
            serde_json::Value::Null => false,
        };

        if ok {
            Ok(())
        } else {
            Err(TemplateValidationFailure::Invalid {
                name: field_name.to_string(),
                expected: self.clone(),
                actual: value.clone(),
            })
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TemplateField {
    pub format: TemplateFieldFormat,
    pub optional: bool,
    pub description: Option<String>,
}

impl TemplateField {
    pub fn from_static(
        format: TemplateFieldFormat,
        optional: bool,
        description: &'static str,
    ) -> TemplateField {
        TemplateField {
            format,
            optional,
            description: Some(description.to_string()),
        }
    }
}

pub type TemplateFields = FxHashMap<String, TemplateField>;

#[derive(Debug)]
pub enum TemplateValidationFailure {
    Required(String),
    Invalid {
        name: String,
        expected: TemplateFieldFormat,
        actual: serde_json::Value,
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

fn apply(
    template: &Vec<(String, serde_json::Value)>,
    values: &FxHashMap<String, serde_json::Value>,
) -> Result<FxHashMap<String, serde_json::Value>, TemplateRenderError> {
    template
        .iter()
        .map(|(name, field_template)| {
            apply_field(field_template, values).map(|rendered| (name.to_string(), rendered))
        })
        .collect::<Result<FxHashMap<_, _>, _>>()
}

pub fn validate_and_apply<'a>(
    object: &'static str,
    id: i64,
    fields: &TemplateFields,
    template: &'a Vec<(String, serde_json::Value)>,
    values: &FxHashMap<String, serde_json::Value>,
) -> Result<FxHashMap<String, serde_json::Value>, TemplateError> {
    validate(object, id, fields, values)?;
    apply(template, values).map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    mod validate {
        use super::super::{validate, TemplateFieldFormat, TemplateValidationFailure};
        use serde_json::{value, Value};
        use std::error::Error;

        #[test]
        fn string() -> Result<(), TemplateValidationFailure> {
            TemplateFieldFormat::String.validate("string", &Value::String(String::new()))?;

            TemplateFieldFormat::String.validate("integer", &serde_json::json!({"a": 5})["a"])?;

            TemplateFieldFormat::String.validate(
                "float",
                &Value::Number(value::Number::from_f64(5.5).unwrap()),
            )?;

            TemplateFieldFormat::String.validate("boolean", &Value::Bool(true))?;

            TemplateFieldFormat::String
                .validate("object", &Value::Object(value::Map::new()))
                .expect_err("object input");

            TemplateFieldFormat::String
                .validate("array", &Value::Array(Vec::new()))
                .expect_err("array input");

            Ok(())
        }

        #[test]
        fn string_array() -> Result<(), TemplateValidationFailure> {
            TemplateFieldFormat::StringArray
                .validate("string", &Value::String(String::new()))
                .expect_err("string");

            TemplateFieldFormat::StringArray
                .validate("integer", &serde_json::json!({"a": 5})["a"])
                .expect_err("integer");

            TemplateFieldFormat::StringArray
                .validate(
                    "float",
                    &Value::Number(value::Number::from_f64(5.5).unwrap()),
                )
                .expect_err("float");

            TemplateFieldFormat::StringArray
                .validate("boolean", &Value::Bool(true))
                .expect_err("boolean");

            TemplateFieldFormat::StringArray
                .validate("object", &Value::Object(value::Map::new()))
                .expect_err("object input");

            TemplateFieldFormat::StringArray.validate("array", &Value::Array(Vec::new()))?;

            Ok(())
        }

        #[test]
        fn integer() -> Result<(), TemplateValidationFailure> {
            TemplateFieldFormat::Integer
                .validate("string", &Value::String(String::new()))
                .expect_err("string");

            TemplateFieldFormat::Integer.validate("integer", &serde_json::json!({"a": 5})["a"])?;

            TemplateFieldFormat::Integer
                .validate(
                    "float",
                    &Value::Number(value::Number::from_f64(5.5).unwrap()),
                )
                .expect_err("float");

            TemplateFieldFormat::Integer
                .validate("boolean", &Value::Bool(true))
                .expect_err("boolean");

            TemplateFieldFormat::Integer
                .validate("object", &Value::Object(value::Map::new()))
                .expect_err("object input");

            TemplateFieldFormat::Integer
                .validate("array", &Value::Array(Vec::new()))
                .expect_err("array input");

            Ok(())
        }

        #[test]
        fn float() -> Result<(), TemplateValidationFailure> {
            TemplateFieldFormat::Float
                .validate("string", &Value::String(String::new()))
                .expect_err("string");

            TemplateFieldFormat::Float.validate("integer", &serde_json::json!({"a": 5})["a"])?;

            TemplateFieldFormat::Float.validate(
                "float",
                &Value::Number(value::Number::from_f64(5.5).unwrap()),
            )?;

            TemplateFieldFormat::Float
                .validate("boolean", &Value::Bool(true))
                .expect_err("boolean");

            TemplateFieldFormat::Float
                .validate("object", &Value::Object(value::Map::new()))
                .expect_err("object input");

            TemplateFieldFormat::Float
                .validate("array", &Value::Array(Vec::new()))
                .expect_err("array input");

            Ok(())
        }

        #[test]
        fn boolean() -> Result<(), TemplateValidationFailure> {
            TemplateFieldFormat::Boolean
                .validate("string", &Value::String(String::new()))
                .expect_err("non-matching string");
            TemplateFieldFormat::Boolean
                .validate("true string", &Value::String("true".to_string()))?;
            TemplateFieldFormat::Boolean
                .validate("false string", &Value::String("false".to_string()))?;

            TemplateFieldFormat::Boolean
                .validate("integer", &serde_json::json!({"a": 5})["a"])
                .expect_err("integer");

            TemplateFieldFormat::Boolean
                .validate(
                    "float",
                    &Value::Number(value::Number::from_f64(5.5).unwrap()),
                )
                .expect_err("float");

            TemplateFieldFormat::Boolean.validate("boolean", &Value::Bool(true))?;

            TemplateFieldFormat::Boolean
                .validate("object", &Value::Object(value::Map::new()))
                .expect_err("object input");

            TemplateFieldFormat::Boolean
                .validate("array", &Value::Array(Vec::new()))
                .expect_err("array input");

            Ok(())
        }

        #[test]
        fn object() -> Result<(), TemplateValidationFailure> {
            TemplateFieldFormat::Object
                .validate("string", &Value::String(String::new()))
                .expect_err("string");

            TemplateFieldFormat::Object
                .validate("integer", &serde_json::json!({"a": 5})["a"])
                .expect_err("integer");

            TemplateFieldFormat::Object
                .validate(
                    "float",
                    &Value::Number(value::Number::from_f64(5.5).unwrap()),
                )
                .expect_err("float");

            TemplateFieldFormat::Object
                .validate("boolean", &Value::Bool(true))
                .expect_err("boolean");

            TemplateFieldFormat::Object.validate("object", &Value::Object(value::Map::new()))?;

            TemplateFieldFormat::Object
                .validate("array", &Value::Array(Vec::new()))
                .expect_err("array input");

            Ok(())
        }

        #[test]
        fn choice_failing_formats() -> Result<(), TemplateValidationFailure> {
            let choice = TemplateFieldFormat::Choice {
                choices: vec!["abc".to_string(), "def".to_string()],
                min: None,
                max: None,
            };

            choice
                .validate("integer", &serde_json::json!({"a": 5})["a"])
                .expect_err("integer");

            choice
                .validate(
                    "float",
                    &Value::Number(value::Number::from_f64(5.5).unwrap()),
                )
                .expect_err("float");

            choice
                .validate("boolean", &Value::Bool(true))
                .expect_err("boolean");

            choice
                .validate("object", &Value::Object(value::Map::new()))
                .expect_err("object input");

            Ok(())
        }

        #[test]
        fn choice_against_string() -> Result<(), TemplateValidationFailure> {
            let choice = TemplateFieldFormat::Choice {
                choices: vec!["abc".to_string(), "def".to_string()],
                min: None,
                max: None,
            };

            choice.validate("matching string", &Value::String("abc".to_string()))?;
            choice
                .validate("non-matching string", &Value::String("jklsdf".to_string()))
                .expect_err("non-matching string");

            let choice = TemplateFieldFormat::Choice {
                choices: vec!["abc".to_string(), "def".to_string()],
                min: Some(1),
                max: None,
            };
            choice.validate("min=1: matching string", &Value::String("abc".to_string()))?;
            choice
                .validate(
                    "min=1: non-matching string",
                    &Value::String("jklsdf".to_string()),
                )
                .expect_err("min=1: non-matching string");

            let choice = TemplateFieldFormat::Choice {
                choices: vec!["abc".to_string(), "def".to_string()],
                min: Some(2),
                max: None,
            };
            choice
                .validate("min=2: matching string", &Value::String("abc".to_string()))
                .expect_err("min=2: matching string");
            choice
                .validate(
                    "min=2: non-matching string",
                    &Value::String("jklsdf".to_string()),
                )
                .expect_err("min=2: non-matching string");

            Ok(())
        }

        #[test]
        fn choice_against_array() -> Result<(), TemplateValidationFailure> {
            fn make_array(v: Vec<&'static str>) -> serde_json::Value {
                let strings = v
                    .iter()
                    .map(|s| serde_json::Value::String(s.to_string()))
                    .collect::<Vec<_>>();
                serde_json::Value::Array(strings)
            }

            let choice = TemplateFieldFormat::Choice {
                choices: vec!["abc".to_string(), "def".to_string(), "ghi".to_string()],
                min: None,
                max: None,
            };

            choice.validate("empty list", &make_array(vec![]))?;
            choice.validate("matching one-element list", &make_array(vec!["def"]))?;
            choice.validate(
                "matching three-element list",
                &make_array(vec!["def", "ghi", "abc"]),
            )?;
            choice
                .validate("non-matching element", &make_array(vec!["def", "jklsdf"]))
                .expect_err("non-matching element");

            let choice = TemplateFieldFormat::Choice {
                choices: vec!["abc".to_string(), "def".to_string(), "ghi".to_string()],
                min: Some(1),
                max: None,
            };

            choice
                .validate("min=1: empty list", &make_array(vec![]))
                .expect_err("min=1: empty list");
            choice.validate("min=1: matching one-element list", &make_array(vec!["def"]))?;
            choice.validate(
                "min=1: matching three-element list",
                &make_array(vec!["def", "ghi", "abc"]),
            )?;
            choice
                .validate(
                    "min=1: non-matching element",
                    &make_array(vec!["def", "jklsdf"]),
                )
                .expect_err("non-matching element");

            let choice = TemplateFieldFormat::Choice {
                choices: vec!["abc".to_string(), "def".to_string(), "ghi".to_string()],
                min: Some(2),
                max: None,
            };

            choice
                .validate("min=2: empty list", &make_array(vec![]))
                .expect_err("min=2: empty list");
            choice
                .validate("min=2: matching one-element list", &make_array(vec!["def"]))
                .expect_err("min=2: matching one-element list");
            choice.validate(
                "min=2: matching two-element list",
                &make_array(vec!["abc", "def"]),
            )?;
            choice.validate(
                "min=2: matching three-element list",
                &make_array(vec!["def", "ghi", "abc"]),
            )?;
            choice
                .validate(
                    "min=2: non-matching element",
                    &make_array(vec!["def", "jklsdf", "abc"]),
                )
                .expect_err("non-matching element");

            let choice = TemplateFieldFormat::Choice {
                choices: vec!["abc".to_string(), "def".to_string(), "ghi".to_string()],
                min: Some(1),
                max: Some(2),
            };

            choice
                .validate("min=1, max=2: empty list", &make_array(vec![]))
                .expect_err("min=1, max=2: empty list");
            choice.validate(
                "min=1, max=2: matching one-element list",
                &make_array(vec!["def"]),
            )?;
            choice.validate(
                "min=1:, max=2 matching two-element list",
                &make_array(vec!["abc", "def"]),
            )?;
            choice
                .validate(
                    "min=1:, max=2 matching three-element list",
                    &make_array(vec!["def", "ghi", "abc"]),
                )
                .expect_err("min=1, max=2: matching three-element list");
            choice
                .validate(
                    "min=1:, max=2 non-matching element",
                    &make_array(vec!["def", "jklsdf"]),
                )
                .expect_err("non-matching element");

            Ok(())
        }
    }

    mod apply {
        use super::super::apply;
        use assert_matches::assert_matches;
        use fxhash::FxHashMap;
        use serde_json::{value, Value};
        use std::{array::IntoIter, error::Error, iter::FromIterator};

        #[test]
        fn simple_template() -> Result<(), anyhow::Error> {
            let template = vec![
                (
                    "command".to_string(),
                    serde_json::Value::String("{{command}}".to_string()),
                ),
                (
                    "output".to_string(),
                    serde_json::Value::String("{{filename}}.json".to_string()),
                ),
            ];

            let values = FxHashMap::<String, serde_json::Value>::from_iter(IntoIter::new([
                (
                    "command".to_string(),
                    serde_json::Value::String("program".to_string()),
                ),
                (
                    "filename".to_string(),
                    serde_json::Value::String("fgh".to_string()),
                ),
                (
                    "extra".to_string(),
                    serde_json::Value::String("extra".to_string()),
                ),
            ]));

            let output = apply(&template, &values)?;

            assert_matches!(output.get("command"), Some(serde_json::Value::String(s)) => {
                assert_eq!(s, "program");
            });

            assert_matches!(output.get("output"), Some(serde_json::Value::String(s)) => {
                assert_eq!(s, "fgh.json");
            });

            let mut keys = output.keys().collect::<Vec<_>>();
            keys.sort();
            assert_eq!(keys, ["command", "output"]);

            Ok(())
        }

        #[test]
        #[ignore]
        fn object_template() {
            todo!()
        }

        #[test]
        #[ignore]
        fn array_template() {
            todo!()
        }
    }
}
