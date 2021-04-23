use crate::{database::PostgresPool, error::Error};

use super::{
    execute::{Executor, ExecutorError},
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};
use async_trait::async_trait;
use fxhash::FxHashMap;

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
    ) -> Result<(), ExecutorError> {
        Ok(())
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}
