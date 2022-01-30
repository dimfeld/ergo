use std::{borrow::Cow, fmt::Display};

use assert_matches::assert_matches;
use ergo_database::sqlx_json_decode;
use fxhash::FxHashMap;
use itertools::Itertools;
use lazy_static::lazy_static;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use tracing::{event, Level};

use super::TaskActionTemplate;

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
    Render(#[from] handlebars::RenderError),
    #[error("Missing value for {0}")]
    MissingValue(String),
}

#[derive(Clone, Debug, JsonSchema, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TemplateFieldFormat {
    String,
    StringArray,
    Integer,
    Float,
    Boolean,
    Object {
        /// If true, the object can have values that are arrays or other objects.
        /// If false, the object's values must all be primitives.
        /// This isn't currently validated but does inform the UI's decisions on
        /// how to proceed.
        #[serde(default)]
        nested: bool,
    },
    Choice {
        choices: Cow<'static, [Cow<'static, str>]>,
        min: Option<usize>,
        max: Option<usize>,
    },
}

impl TemplateFieldFormat {
    pub const fn from_static_choices(
        choices: &'static [Cow<'static, str>],
        min: Option<usize>,
        max: Option<usize>,
    ) -> Self {
        TemplateFieldFormat::Choice {
            choices: Cow::Borrowed(choices),
            min,
            max,
        }
    }

    fn validate(
        &self,
        field_name: &str,
        value: &serde_json::Value,
    ) -> Result<(), TemplateValidationFailure> {
        let ok = match value {
            serde_json::Value::String(s) => {
                if is_payload_template(s) {
                    true
                } else {
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
                                choices.iter().any(|x| x == s)
                            }
                        }
                        _ => false,
                    }
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
                                .any(|c| value.as_str().map(|s| s == c).unwrap_or(false))
                        })
                    }
                }
                _ => false,
            },
            serde_json::Value::Bool(_) => matches!(self, Self::String | Self::Boolean),
            serde_json::Value::Number(n) => match self {
                Self::String | Self::Float => true,
                Self::Integer => n.is_i64(),
                _ => false,
            },
            // TODO If nested is false, validate that the top-level values in the object are all primitives.
            serde_json::Value::Object(_) => matches!(self, &TemplateFieldFormat::Object { .. }),
            serde_json::Value::Null => false,
        };

        if ok {
            Ok(())
        } else {
            Err(TemplateValidationFailure::Invalid {
                name: Cow::from(field_name.to_string()),
                expected: self.clone(),
                actual: value.clone(),
            })
        }
    }
}

impl ToString for TemplateFieldFormat {
    fn to_string(&self) -> String {
        match self {
            Self::String => "string".to_string(),
            Self::StringArray => "array".to_string(),
            Self::Integer => "integer".to_string(),
            Self::Float => "number".to_string(),
            Self::Boolean => "boolean".to_string(),
            Self::Object { .. } => "object".to_string(),
            Self::Choice { choices, min, max } => {
                let choice_strings = choices.iter().join(", ");
                match (min, max) {
                    (Some(1), Some(1)) => format!("One of {}", choice_strings),
                    (Some(min), Some(max)) => format!(
                        "Between {min} and {max} of {choice_strings}",
                        min = min,
                        max = max,
                        choice_strings = choice_strings
                    ),
                    (Some(min), None) => {
                        format!(
                            "At least {min} of {choice_strings}",
                            min = min,
                            choice_strings = choice_strings
                        )
                    }
                    (None, Some(max)) => {
                        format!(
                            "At most {max} of {choice_strings}",
                            max = max,
                            choice_strings = choice_strings
                        )
                    }
                    (None, None) => format!(
                        "Values in {choice_strings}",
                        choice_strings = choice_strings
                    ),
                }
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct TemplateField {
    pub name: Cow<'static, str>,
    pub format: TemplateFieldFormat,
    pub optional: bool,
    pub description: Option<Cow<'static, str>>,
}

impl TemplateField {
    pub const fn from_static(
        name: &'static str,
        format: TemplateFieldFormat,
        optional: bool,
        description: &'static str,
    ) -> TemplateField {
        TemplateField {
            name: Cow::Borrowed(name),
            format,
            optional,
            description: Some(Cow::Borrowed(description)),
        }
    }

    pub fn extract<T: DeserializeOwned>(
        &self,
        payload: &FxHashMap<String, serde_json::Value>,
    ) -> Result<Option<T>, TemplateValidationFailure> {
        let value = payload.get(self.name.as_ref());

        match value {
            Some(v) => serde_json::from_value(v.clone()).map(Some).map_err(|_| {
                TemplateValidationFailure::Invalid {
                    name: self.name.clone(),
                    expected: self.format.clone(),
                    actual: v.clone(),
                }
            }),
            None => {
                if self.optional {
                    Ok(None)
                } else {
                    Err(TemplateValidationFailure::Required(self.name.clone()))
                }
            }
        }
    }

    pub fn extract_str<'a>(
        &self,
        payload: &'a FxHashMap<String, serde_json::Value>,
    ) -> Result<Option<&'a str>, TemplateValidationFailure> {
        match payload.get(self.name.as_ref()) {
            Some(v) => match v {
                serde_json::Value::String(s) => Ok(Some(s.as_str())),
                _ => Err(TemplateValidationFailure::Invalid {
                    name: self.name.clone(),
                    actual: v.clone(),
                    expected: TemplateFieldFormat::String,
                }),
            },
            None => {
                if self.optional {
                    Ok(None)
                } else {
                    Err(TemplateValidationFailure::Required(self.name.clone()))
                }
            }
        }
    }

    pub fn extract_object<'a>(
        &self,
        payload: &'a FxHashMap<String, serde_json::Value>,
    ) -> Result<Option<&'a serde_json::Value>, TemplateValidationFailure> {
        assert_matches!(self.format, TemplateFieldFormat::Object { .. });

        let value = payload.get(self.name.as_ref());
        match value {
            Some(v) => Ok(Some(v)),
            None => {
                if self.optional {
                    Ok(None)
                } else {
                    Err(TemplateValidationFailure::Required(self.name.clone()))
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TemplateFields(pub Vec<TemplateField>);

impl std::ops::Deref for TemplateFields {
    type Target = Vec<TemplateField>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TemplateFields> for Vec<TemplateField> {
    fn from(f: TemplateFields) -> Vec<TemplateField> {
        f.0
    }
}

impl From<Vec<TemplateField>> for TemplateFields {
    fn from(v: Vec<TemplateField>) -> Self {
        TemplateFields(v)
    }
}

#[cfg(not(target_family = "wasm"))]
sqlx_json_decode!(TemplateFields);

#[derive(Debug)]
pub enum TemplateValidationFailure {
    Required(Cow<'static, str>),
    Invalid {
        name: Cow<'static, str>,
        expected: TemplateFieldFormat,
        actual: serde_json::Value,
    },
}

impl std::error::Error for TemplateValidationFailure {}

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

pub fn validate(
    object: &'static str,
    id: Option<impl ToString>,
    fields: &TemplateFields,
    values: &FxHashMap<String, serde_json::Value>,
) -> Result<(), TemplateError> {
    let errors = fields
        .iter()
        .filter_map(
            |field| match (values.get(field.name.as_ref()), field.optional) {
                (Some(v), _) => Some(field.format.validate(field.name.as_ref(), v)),
                (None, true) => None,
                (None, false) => Some(Err(TemplateValidationFailure::Required(
                    field.name.to_owned(),
                ))),
            },
        )
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
            id: id.map(|i| i.to_string()).unwrap_or_else(String::new),
            fields: errors,
        }))
    }
}

fn is_payload_template(template: &str) -> bool {
    template.starts_with("{{/") && template.ends_with("}}")
}

fn apply_field(
    template: &serde_json::Value,
    values: &FxHashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, TemplateError> {
    let result = match template {
        serde_json::Value::String(template) => {
            if is_payload_template(template) {
                // This is ok because we already verified that the skipped set of bytes are
                // ASCII in starts_with and ends_with.
                let field_name = &template[3..template.len() - 2];
                values
                    .get(field_name)
                    .cloned()
                    .ok_or_else(|| TemplateError::MissingValue(field_name.to_string()))?
            } else {
                let rendered = HANDLEBARS.render_template(template, values)?;

                let trimmed = rendered.trim();
                let result = if trimmed.len() == rendered.len() {
                    rendered
                } else {
                    trimmed.to_string()
                };

                serde_json::Value::String(result)
            }
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
                    Ok::<_, TemplateError>((k.clone(), mapped_value))
                })
                .collect::<Result<serde_json::Map<String, _>, _>>()?;
            serde_json::Value::Object(output_object)
        }
        s => s.clone(),
    };

    Ok(result)
}

fn apply(
    template: &TaskActionTemplate,
    values: &FxHashMap<String, serde_json::Value>,
) -> Result<FxHashMap<String, serde_json::Value>, TemplateError> {
    template
        .iter()
        .map(|(name, field_template)| {
            apply_field(field_template, values).map(|rendered| (name.to_string(), rendered))
        })
        .collect::<Result<FxHashMap<_, _>, _>>()
}

pub fn validate_and_apply<'a>(
    object: &'static str,
    id: impl ToString + std::fmt::Display,
    fields: &TemplateFields,
    template: &'a TaskActionTemplate,
    values: &FxHashMap<String, serde_json::Value>,
) -> Result<FxHashMap<String, serde_json::Value>, TemplateError> {
    event!(Level::DEBUG, id=%id, fields=?fields, template=?template, values=?values);
    validate(object, Some(id), fields, values)?;
    apply(template, values)
}

#[cfg(test)]
mod tests {
    mod validate {
        use super::super::{TemplateFieldFormat, TemplateValidationFailure};
        use serde_json::{value, Value};
        use std::borrow::Cow;

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
            TemplateFieldFormat::Object { nested: true }
                .validate("string", &Value::String(String::new()))
                .expect_err("string");

            TemplateFieldFormat::Object { nested: true }
                .validate("integer", &serde_json::json!({"a": 5})["a"])
                .expect_err("integer");

            TemplateFieldFormat::Object { nested: true }
                .validate(
                    "float",
                    &Value::Number(value::Number::from_f64(5.5).unwrap()),
                )
                .expect_err("float");

            TemplateFieldFormat::Object { nested: true }
                .validate("boolean", &Value::Bool(true))
                .expect_err("boolean");

            TemplateFieldFormat::Object { nested: true }
                .validate("object", &Value::Object(value::Map::new()))?;

            TemplateFieldFormat::Object { nested: true }
                .validate("array", &Value::Array(Vec::new()))
                .expect_err("array input");

            Ok(())
        }

        #[test]
        fn choice_failing_formats() -> Result<(), TemplateValidationFailure> {
            let choice = TemplateFieldFormat::Choice {
                choices: Cow::from(vec![Cow::from("abc"), Cow::from("def")]),
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
                choices: Cow::from(vec![Cow::from("abc"), Cow::from("def")]),
                min: None,
                max: None,
            };

            choice.validate("matching string", &Value::String("abc".to_string()))?;
            choice
                .validate("non-matching string", &Value::String("jklsdf".to_string()))
                .expect_err("non-matching string");

            let choice = TemplateFieldFormat::Choice {
                choices: Cow::from(vec![Cow::from("abc"), Cow::from("def")]),
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
                choices: Cow::from(vec![Cow::from("abc"), Cow::from("def")]),
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
                choices: Cow::from(vec![Cow::from("abc"), Cow::from("def"), Cow::from("ghi")]),
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
                choices: Cow::from(vec![Cow::from("abc"), Cow::from("def"), Cow::from("ghi")]),
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
                choices: Cow::from(vec![Cow::from("abc"), Cow::from("def"), Cow::from("ghi")]),
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
                choices: Cow::from(vec![Cow::from("abc"), Cow::from("def"), Cow::from("ghi")]),
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
        use serde_json::json;
        use std::{array::IntoIter, iter::FromIterator};

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
        fn full_object_template() -> Result<(), anyhow::Error> {
            let template = vec![(
                "payload".to_string(),
                serde_json::Value::String("{{/json}}".to_string()),
            )];

            let values = std::array::IntoIter::new([(
                "json".to_string(),
                serde_json::json!({"a": 5, "b": "c"}),
            )])
            .into_iter()
            .collect::<FxHashMap<_, _>>();

            let output = apply(&template, &values)?;
            assert_eq!(
                output.get("payload"),
                Some(&serde_json::json!({"a": 5, "b": "c"}))
            );

            Ok(())
        }

        #[test]
        fn complex_template() -> Result<(), anyhow::Error> {
            let template = vec![
                (
                    "body".to_string(),
                    json!({
                        "message": "{{method}}",
                        "options": {
                            "a": "{{a_value}}",
                            "b": "{{b_value}}"
                        },
                        "scopes": [
                            "general",
                            "{{scopes/a}}",
                            "{{scopes/b}}"
                        ]
                    }),
                ),
                (
                    "output".to_string(),
                    json!(["{{outputs/a}}", "{{outputs/b}}"]),
                ),
            ];

            let values = FxHashMap::<String, serde_json::Value>::from_iter(IntoIter::new([
                (
                    "outputs".to_string(),
                    json!({
                        "a": "output a",
                        "b": "output b",
                    }),
                ),
                (
                    "scopes".to_string(),
                    json!({
                        "a": "scope a",
                        "b" : "another scope"
                    }),
                ),
                (
                    "method".to_string(),
                    serde_json::Value::String("the method".to_string()),
                ),
                ("a_value".to_string(), json!("option a value")),
                ("b_value".to_string(), json!("option b value")),
            ]));

            let output = apply(&template, &values)?;

            assert_matches!(output.get("body"), Some(o) => {
                assert_eq!(o, &json!({
                    "message": "the method",
                    "options": {
                        "a": "option a value",
                        "b": "option b value"
                    },
                    "scopes": ["general", "scope a", "another scope"],
                }));
            });

            assert_matches!(output.get("output"), Some(o) => {
                assert_eq!(o, &json!(["output a", "output b"]));
            });

            let mut keys = output.keys().collect::<Vec<_>>();
            keys.sort();
            assert_eq!(keys, ["body", "output"]);

            Ok(())
        }
    }
}
