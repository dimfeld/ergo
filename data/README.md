This directory contains prefilled inputs, tasks, and actions that can populate the Ergo database.

For simple use, you can populate the database like so:

1. Run `bootstrap.sh` to add an initial organization and user.
2. Create an API key using `cargo run --bin make_api_key`. Set the resulting API key to the API_KEY variable. Note that this
API key is not retrievable after creation, so be sure to save it somewhere.
3. Run `load_input.sh inputs/*.json` to load all the premade inputs.
4. Run `load_action.sh actions/*.json` to load all the premade actions.
