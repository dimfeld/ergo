use std::borrow::Cow;

use super::{
    execute::{Executor, ExecutorError},
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};
use anyhow::anyhow;
use async_trait::async_trait;
use fxhash::FxHashMap;
use serde_json::json;
use tracing::{event, instrument, Level};

static FIELD_URL: TemplateField = TemplateField::from_static(
    "url",
    TemplateFieldFormat::string_without_default(),
    false,
    "The URL to request",
);

static FIELD_METHOD: TemplateField = TemplateField::from_static(
    "method",
    TemplateFieldFormat::from_static_choices(
        &[
            Cow::Borrowed("GET"),
            Cow::Borrowed("POST"),
            Cow::Borrowed("PUT"),
            Cow::Borrowed("PATCH"),
            Cow::Borrowed("DELETE"),
            Cow::Borrowed("HEAD"),
            Cow::Borrowed("OPTIONS"),
        ],
        Some(1),
        Some(1),
        &[Cow::Borrowed("GET")],
    ),
    true,
    "The HTTP method to use. Defaults to GET",
);

static FIELD_USER_AGENT: TemplateField = TemplateField::from_static(
    "user_agent",
    TemplateFieldFormat::String {
        default: Cow::Borrowed("Ergo"),
    },
    true,
    "Use a custom user agent string (default is 'Ergo')",
);

static FIELD_TIMEOUT: TemplateField = TemplateField::from_static(
    "timeout",
    TemplateFieldFormat::Integer { default: 30 },
    true,
    "The request timeout, in seconds. Default is 30 seconds",
);

static FIELD_JSON: TemplateField = TemplateField::from_static(
    "json",
    TemplateFieldFormat::Object {
        nested: true,
        default: Cow::Borrowed("{}"),
    },
    true,
    "A JSON body to send with the request",
);

static FIELD_BODY: TemplateField = TemplateField::from_static(
    "body",
    TemplateFieldFormat::string_without_default(),
    true,
    "A raw string body to send with the request",
);

static FIELD_QUERY: TemplateField = TemplateField::from_static(
    "query",
    TemplateFieldFormat::object_without_default(false),
    true,
    "Query string to send",
);

static FIELD_HEADERS: TemplateField = TemplateField::from_static(
    "headers",
    TemplateFieldFormat::Object {
        nested: false,
        default: Cow::Borrowed(""),
    },
    true,
    "HTTP header values for the request",
);

static FIELD_RESULT_FORMAT: TemplateField = TemplateField::from_static(
    "result_format",
    TemplateFieldFormat::from_static_choices(
        &[Cow::Borrowed("json"), Cow::Borrowed("string")],
        Some(1),
        Some(1),
        &[Cow::Borrowed("json")],
    ),
    true,
    "How to process the result. Defaults to JSON",
);

#[derive(Debug)]
pub struct HttpExecutor {
    template_fields: TemplateFields,
}

impl HttpExecutor {
    pub fn new() -> HttpExecutor {
        let template_fields = [
            &FIELD_URL,
            &FIELD_METHOD,
            &FIELD_USER_AGENT,
            &FIELD_TIMEOUT,
            &FIELD_JSON,
            &FIELD_BODY,
            &FIELD_QUERY,
            &FIELD_HEADERS,
            &FIELD_RESULT_FORMAT,
        ]
        .into();

        HttpExecutor { template_fields }
    }
}

#[async_trait]
impl Executor for HttpExecutor {
    fn name(&self) -> &'static str {
        "http"
    }

    #[cfg(not(target_family = "wasm"))]
    #[instrument(level = "debug", name = "HttpExecutor::execute", skip(_state))]
    async fn execute(
        &self,
        _state: super::execute::ExecutorState,
        payload: FxHashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, ExecutorError> {
        let user_agent = FIELD_USER_AGENT.extract_str(&payload)?;
        let timeout: u64 = FIELD_TIMEOUT.extract(&payload)?;
        let client = reqwest::ClientBuilder::new()
            .user_agent(user_agent.as_ref())
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .map_err(ExecutorError::command_error_without_result)?;

        let method_choice = FIELD_METHOD
            .extract_string_array(&payload)?
            .drain(..)
            .next()
            .unwrap_or(Cow::Borrowed("GET"));
        let method = reqwest::Method::try_from(method_choice.as_ref()).map_err(|_| {
            ExecutorError::FieldFormatError {
                field: "method".to_string(),
                subfield: None,
                expected: "Valid HTTP method".to_string(),
            }
        })?;

        let url = FIELD_URL.extract_str(&payload)?;

        let req = client.request(method, url.as_ref());

        let req = match payload.get("headers") {
            Some(serde_json::Value::Object(o)) => {
                let header_map = o
                    .iter()
                    .map(|(k, v)| {
                        let name = reqwest::header::HeaderName::try_from(k).map_err(|_| {
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

        let query = FIELD_QUERY.extract_object(&payload)?;
        let req = if query.is_object() {
            req.query(&query)
        } else {
            req
        };

        let body = payload
            .get("body")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        event!(Level::INFO, ?req, ?body, "sending request");

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

        let status = result.status().as_u16();

        let output_format = FIELD_RESULT_FORMAT
            .extract_string_array(&payload)?
            .drain(..)
            .next()
            .unwrap_or(Cow::Borrowed("json"));

        let output = if output_format == "string" {
            let r = result
                .text()
                .await
                .map_err(|e| ExecutorError::CommandError {
                    source: anyhow!(e),
                    result: json!(null),
                })?;
            json!({ "response": r, "status": status })
        } else {
            let r = result.json::<serde_json::Value>().await.map_err(|e| {
                ExecutorError::CommandError {
                    source: anyhow!(e),
                    result: json!(null),
                }
            })?;
            json!({ "response": r, "status": status })
        };

        Ok(output)
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::execute::ExecutorState;

    use super::*;
    use assert_matches::assert_matches;
    use serde_json::json;
    use wiremock::{
        matchers::{self, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn simple_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/a_url"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("the response")))
            .mount(&mock_server)
            .await;

        let payload =
            std::array::IntoIter::new([("url", json!(format!("{}/a_url", mock_server.uri())))])
                .map(|(k, v)| (k.to_string(), v))
                .collect::<FxHashMap<String, serde_json::Value>>();
        let exec = HttpExecutor::new();

        let result = exec
            .execute(ExecutorState::new_test_state(), payload)
            .await
            .expect("Running action");

        assert_eq!(result, json!({"response": "the response", "status": 200 }));
    }

    #[tokio::test]
    async fn complex_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/a_url"))
            .and(matchers::query_param("qq", "rr"))
            .and(matchers::header("a-custom-header", "abc"))
            .and(matchers::header("user-agent", "a custom user agent"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("the response")))
            .mount(&mock_server)
            .await;

        let payload = std::array::IntoIter::new([
            ("url", json!(format!("{}/a_url", mock_server.uri()))),
            ("method", json!("POST")),
            ("query", json!({ "qq": "rr" })),
            ("headers", json!({"a-custom-header": "abc"})),
            ("user_agent", json!("a custom user agent")),
        ])
        .map(|(k, v)| (k.to_string(), v))
        .collect::<FxHashMap<_, _>>();
        let exec = HttpExecutor::new();

        let result = exec
            .execute(ExecutorState::new_test_state(), payload)
            .await
            .expect("Running action");

        assert_eq!(result, json!({"response": "the response", "status": 200 }));
    }

    #[tokio::test]
    async fn string_body() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/a_url"))
            .and(matchers::body_string("this is a string body"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("the response")))
            .mount(&mock_server)
            .await;

        let payload = std::array::IntoIter::new([
            ("url", json!(format!("{}/a_url", mock_server.uri()))),
            ("method", json!("POST")),
            ("body", json!("this is a string body")),
        ])
        .map(|(k, v)| (k.to_string(), v))
        .collect::<FxHashMap<_, _>>();
        let exec = HttpExecutor::new();

        let result = exec
            .execute(ExecutorState::new_test_state(), payload)
            .await
            .expect("Running action");

        assert_eq!(result, json!({"response": "the response", "status": 200 }));
    }

    #[tokio::test]
    async fn json_body() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/a_url"))
            .and(matchers::body_json(json!({"a": 4, "b": "c"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("the response")))
            .mount(&mock_server)
            .await;

        let payload = std::array::IntoIter::new([
            ("url", json!(format!("{}/a_url", mock_server.uri()))),
            ("method", json!("POST")),
            ("json", json!({"a": 4, "b": "c"})),
        ])
        .map(|(k, v)| (k.to_string(), v))
        .collect::<FxHashMap<_, _>>();
        let exec = HttpExecutor::new();

        let result = exec
            .execute(ExecutorState::new_test_state(), payload)
            .await
            .expect("Running action");

        assert_eq!(result, json!({"response": "the response", "status": 200 }));
    }

    #[tokio::test]
    async fn string_result() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/a_url"))
            .respond_with(ResponseTemplate::new(202).set_body_string("a string response"))
            .mount(&mock_server)
            .await;

        let payload = std::array::IntoIter::new([
            ("url", json!(format!("{}/a_url", mock_server.uri()))),
            ("method", json!("GET")),
            ("result_format", json!(["string"])),
        ])
        .map(|(k, v)| (k.to_string(), v))
        .collect::<FxHashMap<_, _>>();
        let exec = HttpExecutor::new();

        let result = exec
            .execute(ExecutorState::new_test_state(), payload)
            .await
            .expect("Running action");

        assert_eq!(
            result,
            json!({"response": "a string response", "status": 202 })
        );
    }

    #[tokio::test]
    async fn json_result() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/a_url"))
            .and(matchers::body_json(json!({"a": 4, "b": "c"})))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"response_json": 5, "another_key": 6})),
            )
            .mount(&mock_server)
            .await;

        let payload = std::array::IntoIter::new([
            ("url", json!(format!("{}/a_url", mock_server.uri()))),
            ("method", json!(["POST"])),
            ("json", json!({"a": 4, "b": "c"})),
            ("result_format", json!(["json"])),
        ])
        .map(|(k, v)| (k.to_string(), v))
        .collect::<FxHashMap<_, _>>();
        let exec = HttpExecutor::new();

        let result = exec
            .execute(ExecutorState::new_test_state(), payload)
            .await
            .expect("Running action");

        assert_eq!(
            result,
            json!({"response": { "response_json": 5, "another_key": 6}, "status": 200 })
        );
    }

    #[tokio::test]
    async fn error_result() {
        // Start a server that doesn't match on anything.
        let mock_server = MockServer::start().await;

        let payload = std::array::IntoIter::new([
            ("url", json!(format!("{}/a_url", mock_server.uri()))),
            ("method", json!("POST")),
        ])
        .map(|(k, v)| (k.to_string(), v))
        .collect::<FxHashMap<_, _>>();
        let exec = HttpExecutor::new();

        let result = exec
            .execute(ExecutorState::new_test_state(), payload)
            .await
            .expect_err("Running action");

        assert_matches!(result, ExecutorError::CommandError { .. });
    }
}
