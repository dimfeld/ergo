use super::{
    execute::Executor,
    template::{TemplateField, TemplateFieldFormat, TemplateFields},
};

use async_trait::async_trait;

const FIELD_TASK: TemplateField = TemplateField::from_static(
    "task",
    TemplateFieldFormat::String,
    false,
    "The task to send the input to",
);

const FIELD_TRIGGER: TemplateField = TemplateField::from_static(
    "trigger_name",
    TemplateFieldFormat::String,
    false,
    "The local ID of the task's trigger",
);

const FIELD_TIME: TemplateField = TemplateField::from_static(
    "time",
    TemplateFieldFormat::String,
    true,
    "When to send the input",
);

#[derive(Debug)]
pub struct SendInputExecutor {
    template_fields: TemplateFields,
}

impl SendInputExecutor {
    pub fn new() -> SendInputExecutor {
        let template_fields = vec![FIELD_TRIGGER, FIELD_TASK, FIELD_TIME].into();
        SendInputExecutor { template_fields }
    }
}

#[async_trait]
impl Executor for SendInputExecutor {
    async fn execute(
        &self,
        pg_pool: ergo_database::PostgresPool,
        template_values: fxhash::FxHashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, super::execute::ExecutorError> {
        todo!()
    }

    fn name(&self) -> &'static str {
        "send_input"
    }

    fn template_fields(&self) -> &TemplateFields {
        &self.template_fields
    }
}
