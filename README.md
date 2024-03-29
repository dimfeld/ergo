Ergo is a low-code IFTTT/Zapier style application, built with Rust and Svelte. Tasks are customizable with Javascript and can consist of state machines or DAG-like workflows.

# Considered use cases

- Scrape web sources and send emails based on some content in them
- Receive a URL, run youtube-dl and save the video locally
- Periodically download Roam database
- Some sort of Twitter list archiving?
- Scrape websites and populate a local database
- Fetch filled brokerage orders
- Interactive Slack/Discord bots
- Multi-stage ETL pipelines

# Anticipated Features

This is somewhat out of date.

- [ ] Inputs
  - [X] from POST to an endpoint
  - [ ] send events based on some periodic check that triggers when it sees a condition
  - [X] trigger events unconditionally on a schedule
- [ ] Actions
  - [ ] Spawn docker containers (and/or Nomad jobs?)
  - [X] Query HTTP endpoints
  - [X] Run some local command
  - [X] Link actions to accounts when required
  - [X] Embedded JavaScript for actions
  - [ ] Return data for use by the next task in the chain
- [ ] Data Schemas
  - [X] Each action can specify the types of data that it accepts
  - [X] Duck typing for events
- [ ] State machines will take data from an event, modify it somehow, and pass it on
  - [X] Embed JavaScript to write state machine logic
  - [ ] Persistent context for state machines
  - [ ] Optional schema input
  - [ ] Optional type checking on state machine return value
- [ ] Tasks
  - [X] Trigger a task based on events
  - [X] Run actions
  - [X] Run state machines in response to events
  - [ ] Sequences - Tasks can be chained together and optionally pass information between them (file locations, etc.)
  - [ ] Tasks can clone themselves, and further inputs for that clone are routed to them. This will probably involve interaction with another process set up to be aware of how this works.
- [X] Templates for events, actions, and state machines
- [X] Extensive logging of events, actions, etc.

# Roadmap

## 0.4

- [ ] Tasks made up of linked blocks that run as a DAG

## 0.3

- [X] MVP of UI for editing tasks
- [X] MVP of UI for editing actions and inputs.
- [X] Periodic tasks 
- [X] Task logic can be written in Javascript.
- [X] Implement last missing pieces of queue behavior.

## 0.2

- [X] Add all missing tests
- [X] Read-only Web UI with events timeline
- [X] JavaScript execution as part of state machines, actions, and inputs.
- [X] Set up foundations of serialized long-running tasks in V8.

## 0.1

- [X] Events can be triggered by REST endpoint
- [X] Tasks consists of one event that triggers one or more actions
- [X] Actions can run local scripts
- [X] Log everything that happens
    - [X] Log inputs and actions to Postgres tables
    - [X] Trigger Discord webhooks
- [X] Events go into a queue for processing
- [X] Actions are triggered from a queue

