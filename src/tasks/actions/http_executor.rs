use std::{convert::TryFrom, str::FromStr};

use crate::{database::PostgresPool, error::Error};

use super::{
    execute::{Executor, ExecutorError},
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};
use anyhow::anyhow;
use async_trait::async_trait;
use fxhash::FxHashMap;
use serde_json::json;

#[derive(Debug)]
pub struct HttpExecutor {
    template_fields: TemplateFields,
}

impl HttpExecutor {
    pub fn new() -> (String, Box<dyn Executor>) {
        let template_fields = vec![
            (
                "url",
                TemplateField::from_static(
                    TemplateFieldFormat::String,
                    false,
                    "The URL to request",
                ),
            ),
            (
                "method",
                TemplateField::from_static(
                    TemplateFieldFormat::String,
                    true,
                    "The HTTP method to use. Defaults to GET",
                ),
            ),
            (
                "user_agent",
                TemplateField::from_static(
                    TemplateFieldFormat::String,
                    true,
                    "Use a custom user agent string (default is 'Ergo')",
                ),
            ),
            (
                "timeout",
                TemplateField::from_static(
                    TemplateFieldFormat::Integer,
                    true,
                    "The request timeout, in seconds. Default is 30 seconds",
                ),
            ),
            (
                "json",
                TemplateField::from_static(
                    TemplateFieldFormat::Object,
                    true,
                    "A JSON body to send with the request",
                ),
            ),
            (
                "body",
                TemplateField::from_static(
                    TemplateFieldFormat::String,
                    true,
                    "A raw string body to send with the request",
                ),
            ),
            (
                "query",
                TemplateField::from_static(
                    TemplateFieldFormat::Object,
                    true,
                    "Query string to send",
                ),
            ),
            (
                "headers",
                TemplateField::from_static(
                    TemplateFieldFormat::Object,
                    true,
                    "HTTP header values for the request",
                ),
            ),
            (
                "result_as_bytes",
                TemplateField::from_static(
                    TemplateFieldFormat::Boolean,
                    true,
                    "Treat the result as raw bytes instead of JSON",
                ),
            ),
        ]
        .into_iter()
        .map(|(key, val)| (key.to_string(), val))
        .collect::<TemplateFields>();

        (
            "http".to_string(),
            Box::new(HttpExecutor { template_fields }),
        )
    }
}

#[async_trait]
impl Executor for HttpExecutor {
    async fn execute(
        &self,
        pg_pool: PostgresPool,
        payload: FxHashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, ExecutorError> {
        let user_agent = match payload.get("user_agent") {
            Some(serde_json::Value::String(s)) => s,
            _ => "Ergo",
        };

        let timeout = match payload.get("timeout") {
            Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(30),
            _ => 30,
        };

        let client = reqwest::ClientBuilder::new()
            .user_agent(user_agent)
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .map_err(|e| ExecutorError::CommandError {
                source: anyhow!(e),
                result: serde_json::Value::Null,
            })?;

        let method = reqwest::Method::try_from(match payload.get("method") {
            Some(serde_json::Value::String(s)) => s,
            _ => "GET",
        })
        .map_err(|_| ExecutorError::FieldFormatError {
            field: "method".to_string(),
            subfield: None,
            expected: "Valid HTTP method".to_string(),
        })?;

        let url = payload.get("url").and_then(|u| u.as_str()).ok_or_else(|| {
            ExecutorError::FieldFormatError {
                field: "url".to_string(),
                subfield: None,
                expected: "Request URL".to_string(),
            }
        })?;

        let req = client.request(method, url);

        let req = match payload.get("headers") {
            Some(serde_json::Value::Object(o)) => {
                let header_map = o
                    .iter()
                    .map(|(k, v)| {
                        let name = reqwest::header::HeaderName::try_from(k).map_err(|e| {
                            ExecutorError::FieldFormatError {
                                field: "headers".to_string(),
                                subfield: Some(k.to_string()),
                                expected: "Valid HTTP header name".to_string(),
                            }
                        })?;

                        let value = v
                            .as_str()
                            .and_then(|s| reqwest::header::HeaderValue::from_str(s).ok())
                            .ok_or_else(|| ExecutorError::FieldFormatError {
                                field: "headers".to_string(),
                                subfield: Some(k.to_string()),
                                expected: "Valid HTTP header string value".to_string(),
                            })?;

                        Ok((name, value))
                    })
                    .collect::<Result<reqwest::header::HeaderMap, ExecutorError>>()?;

                req.headers(header_map)
            }
            _ => req,
        };

        let req = match payload.get("query") {
            Some(serde_json::Value::Object(o)) => req.query(o),
            _ => req,
        };

        let body = payload
            .get("body")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let req = match (payload.get("json"), body) {
            (Some(json), _) => req.json(json),
            (None, Some(body)) => req.body(body),
            _ => req,
        };

        let result = req
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| ExecutorError::CommandError {
                source: anyhow!(e),
                result: json!(null),
            })?;

        let output = if payload
            .get("result_as_bytes")
            .and_then(|b| b.as_bool())
            .unwrap_or(false)
        {
            let r = result
                .json()
                .await
                .map_err(|e| ExecutorError::CommandError {
                    source: anyhow!(e),
                    result: json!(null),
                })?;
            json!({ "response": r })
        } else {
            let r = result
                .text()
                .await
                .map_err(|e| ExecutorError::CommandError {
                    source: anyhow!(e),
                    result: json!(null),
                })?;
            json!({ "response": r })
        };

        Ok(output)
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn simple_request() {}

    #[test]
    #[ignore]
    fn complex_request() {}

    #[test]
    #[ignore]
    fn string_body() {}

    #[test]
    #[ignore]
    fn json_body() {}

    #[test]
    #[ignore]
    fn string_result() {}

    #[test]
    #[ignore]
    fn json_result() {}
}
