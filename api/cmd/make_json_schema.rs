use crate::routes::{
    actions::ExecutorInfo,
    inputs::InputPayload,
    tasks::{InputsLogEntry, TaskDescription, TaskInput, TaskResult},
};

use ergo_tasks::{
    actions::{
        execute::ScriptOrTemplate,
        template::{TemplateField, TemplateFieldFormat},
        Action, ActionCategory,
    },
    inputs::Input,
    state_machine::{
        ActionInvokeDef, ActionInvokeDefDataField, ActionPayloadBuilder, EventHandler,
        StateDefinition, StateMachine, StateMachineData, TransitionCondition, TransitionTarget,
    },
};

use schemars::{schema::RootSchema, schema_for};

fn write(dir: &std::path::Path, name: &str, schema: &RootSchema) -> std::io::Result<()> {
    let output = serde_json::to_string_pretty(schema).unwrap();
    let output_path = dir.join(format!("{}.json", name));
    std::fs::write(output_path, output)?;
    Ok(())
}

pub fn main() -> crate::error::Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("schemas");
    let e = std::fs::DirBuilder::new().create(&dir);
    if let Err(e) = e {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(e.into());
        }
    }

    let schema = schema_for!(ExecutorInfo);
    write(&dir, "executor_info", &schema)?;

    let schema = schema_for!(TaskDescription);
    write(&dir, "task_description", &schema)?;

    let schema = schema_for!(TaskInput);
    write(&dir, "task_input", &schema)?;

    let schema = schema_for!(TaskResult);
    write(&dir, "task_result", &schema)?;

    let schema = schema_for!(TemplateField);
    write(&dir, "template_field", &schema)?;

    let schema = schema_for!(TemplateFieldFormat);
    write(&dir, "template_field_format", &schema)?;

    let schema = schema_for!(Action);
    write(&dir, "action", &schema)?;

    let schema = schema_for!(ActionPayloadBuilder);
    write(&dir, "action_payload_builder", &schema)?;

    let schema = schema_for!(ActionInvokeDef);
    write(&dir, "action_invoke_def", &schema)?;

    let schema = schema_for!(ActionInvokeDefDataField);
    write(&dir, "action_invoke_def_data_field", &schema)?;

    let schema = schema_for!(ActionCategory);
    write(&dir, "action_category", &schema)?;

    let schema = schema_for!(ScriptOrTemplate);
    write(&dir, "script_or_template", &schema)?;

    let schema = schema_for!(StateMachineData);
    write(&dir, "state_machine_data", &schema)?;

    let schema = schema_for!(StateDefinition);
    write(&dir, "state_definition", &schema)?;

    let schema = schema_for!(EventHandler);
    write(&dir, "event_handler", &schema)?;

    let schema = schema_for!(TransitionTarget);
    write(&dir, "transition_target", &schema)?;

    let schema = schema_for!(TransitionCondition);
    write(&dir, "transition_condition", &schema)?;

    let schema = schema_for!(StateMachine);
    write(&dir, "state_machine", &schema)?;

    let schema = schema_for!(Input);
    write(&dir, "input", &schema)?;

    let schema = schema_for!(InputPayload);
    write(&dir, "input_payload", &schema)?;

    let schema = schema_for!(InputsLogEntry);
    write(&dir, "inputs_log_schema", &schema)?;

    Ok(())
}
