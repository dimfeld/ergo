use std::{convert::TryFrom, str::FromStr};

use crate::{database::PostgresPool, error::Error};

use super::{
    execute::{Executor, ExecutorError},
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};
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
                TemplateField {
                    format: TemplateFieldFormat::String,
                    optional: false,
                    description: Some("The URL to request".to_string()),
                },
            ),
            (
                "method",
                TemplateField {
                    format: TemplateFieldFormat::String,
                    optional: true,
                    description: Some("The HTTP method to use. Defaults to GET".to_string()),
                },
            ),
            (
                "user_agent",
                TemplateField {
                    format: TemplateFieldFormat::String,
                    optional: true,
                    description: Some(
                        "Use a custom user agent string (default is 'Ergo')".to_string(),
                    ),
                },
            ),
            (
                "timeout",
                TemplateField {
                    format: TemplateFieldFormat::Integer,
                    optional: true,
                    description: Some(
                        "The request timeout, in seconds. Default is 30 seconds".to_string(),
                    ),
                },
            ),
            (
                "json",
                TemplateField {
                    format: TemplateFieldFormat::Object,
                    optional: true,
                    description: Some("A JSON body to send with the request".to_string()),
                },
            ),
            (
                "body",
                TemplateField {
                    format: TemplateFieldFormat::String,
                    optional: true,
                    description: Some("A raw string body to send with the request".to_string()),
                },
            ),
            (
                "query",
                TemplateField {
                    format: TemplateFieldFormat::Object,
                    optional: true,
                    description: Some("Query string to send"),
                },
            ),
            (
                "headers",
                TemplateField {
                    format: TemplateFieldFormat::Object,
                    optional: true,
                    description: Some("HTTP header values for the request".to_string()),
                },
            ),
            (
                "cookies",
                TemplateField {
                    format: TemplateFieldFormat::Object,
                    optional: true,
                    description: Some("HTTP cookies to send with the request".to_string()),
                },
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
                source: e,
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
                let mut header_map = reqwest::header::HeaderMap::try_from(o).map_err(|e| {
                    ExecutorError::FieldFormatError {
                        field: "headers".to_string(),
                        subfield: None,
                        expected: e.to_string(),
                    }
                })?;
                req.headers(header_map)
            }
            _ => req,
        };

        let req = match payload.get("query") {
            Some(serde_json::Value::Object(o)) => req.query(o),
            _ => req,
        };

        let req = match (payload.get("json"), payload.get("body")) {
            (Some(json), _) => req.json(json),
            (None, Some(serde_json::Value::String(body))) => req.body(body),
            _ => req,
        };

        let result = req.send().await;

        // TODO Add cookies, basic/bearer auth

        Ok(json!(null))
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}
