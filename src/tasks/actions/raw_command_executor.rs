use crate::{database::PostgresPool, error::Error};

use super::{
    execute::Executor,
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};
use async_trait::async_trait;
use fxhash::FxHashMap;

#[derive(Debug)]
pub struct RawCommandExecutor {
    template_fields: TemplateFields,
}

impl RawCommandExecutor {
    pub fn new() -> (String, Box<dyn Executor>) {
        let template_fields = vec![
            (
                "command",
                TemplateField {
                    format: TemplateFieldFormat::String,
                    optional: false,
                    description: Some("The executable to run".to_string()),
                },
            ),
            (
                "args",
                TemplateField {
                    format: TemplateFieldFormat::StringArray,
                    optional: true,
                    description: Some("An array of arguments to the executable".to_string()),
                },
            ),
            (
                "env",
                TemplateField {
                    format: TemplateFieldFormat::Object,
                    optional: true,
                    description: Some("Environment variables to set".to_string()),
                },
            ),
        ]
        .into_iter()
        .map(|(key, val)| (key.to_string(), val))
        .collect::<TemplateFields>();

        (
            "raw_command".to_string(),
            Box::new(RawCommandExecutor { template_fields }),
        )
    }
}

#[async_trait]
impl Executor for RawCommandExecutor {
    async fn execute(
        &self,
        pg_pool: PostgresPool,
        payload: FxHashMap<String, serde_json::Value>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}
