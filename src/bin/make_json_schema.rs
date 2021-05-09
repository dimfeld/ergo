use ergo::tasks::{
    actions::handlers::ActionPayload, handlers::TaskInput, inputs::handlers::InputPayload,
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

fn main() -> std::io::Result<()> {
    let task_input = schema_for!(TaskInput);
    write("task_input", &task_input)?;

    let action_payload = schema_for!(ActionPayload);
    write("action_payload", &action_payload)?;
    let input_payload = schema_for!(InputPayload);
    write("input_payload", &input_payload)?;

    Ok(())
}
