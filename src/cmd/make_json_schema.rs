use crate::tasks::{
    actions::{
        execute::ScriptOrTemplate,
        handlers::ActionPayload,
        template::{TemplateField, TemplateFieldFormat},
    },
    handlers::TaskInput,
    inputs::{handlers::InputPayload, Input},
    state_machine::{
        ActionInvokeDef, ActionInvokeDefDataField, ActionPayloadBuilder, EventHandler,
        StateDefinition, StateMachine, StateMachineData, TransitionCondition, TransitionTarget,
    },
};

use schemars::{schema::RootSchema, schema_for};

fn write(name: &str, schema: &RootSchema) -> std::io::Result<()> {
    let output = serde_json::to_string_pretty(schema).unwrap();
    let output_path = std::env::current_dir()
        .unwrap()
        .join("schemas")
        .join(format!("{}.json", name));
    std::fs::write(output_path, output)?;
    Ok(())
}

pub fn main() -> crate::error::Result<()> {
    let dirs = std::env::current_dir().unwrap().join("schemas");
    let e = std::fs::DirBuilder::new().create(&dirs);
    if let Err(e) = e {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(e.into());
        }
    }

    let schema = schema_for!(TaskInput);
    write("task_input", &schema)?;

    let schema = schema_for!(TemplateField);
    write("template_field", &schema)?;

    let schema = schema_for!(TemplateFieldFormat);
    write("template_field_format", &schema)?;

    let schema = schema_for!(ActionPayload);
    write("action_payload", &schema)?;

    let schema = schema_for!(ActionPayloadBuilder);
    write("action_payload_builder", &schema)?;

    let schema = schema_for!(ActionInvokeDef);
    write("action_invoke_def", &schema)?;

    let schema = schema_for!(ActionInvokeDefDataField);
    write("action_invoke_def_data_field", &schema)?;

    let schema = schema_for!(ScriptOrTemplate);
    write("script_or_template", &schema)?;

    let schema = schema_for!(StateMachineData);
    write("state_machine_data", &schema)?;

    let schema = schema_for!(StateDefinition);
    write("state_definition", &schema)?;

    let schema = schema_for!(EventHandler);
    write("event_handler", &schema)?;

    let schema = schema_for!(TransitionTarget);
    write("transition_target", &schema)?;

    let schema = schema_for!(TransitionCondition);
    write("transition_condition", &schema)?;

    let schema = schema_for!(StateMachine);
    write("state_machine", &schema)?;

    let schema = schema_for!(Input);
    write("input", &schema)?;

    let schema = schema_for!(InputPayload);
    write("input_payload", &schema)?;

    Ok(())
}
