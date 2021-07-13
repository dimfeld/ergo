Ergo is a low-code IFTTT/Zapier style application, built with Rust and Svelte. Tasks are customizable with Javascript and can contain state machines for more advanced task handling.

This project is in very early stages right now with no current plans for proper publishing, but that will probably come some day.

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

- [ ] Inputs
  - [X] from POST to an endpoint
  - [ ] some events are sent based on some periodic check that triggers when it sees a condition
  - [ ] trigger events unconditionally on a schedule
- [ ] Actions
  - [ ] Spawn docker containers (and/or Nomad jobs?)
  - [X] Query HTTP endpoints
  - [X] Run some local command
  - [X] Link actions to accounts when required
  - [ ] Embedded scripting for actions
  - [ ] Return data for use by the next task in the chain
- [ ] Data Schemas
  - [X] Each action can specify the types of data that it accepts
  - [X] Duck typing for events
- [ ] State machines will take data from an event, modify it somehow, and pass it on
  - [ ] Embed JavaScript to write state machine logic
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

## 0.2

- [X] Add all missing tests
- [ ] Simple Web UI - in progress
- [ ] Scripting
- [ ] Ability to run Nomad jobs as actions

## 0.1

- [X] Events can be triggered by REST endpoint
- [X] Tasks consists of one event that triggers one or more actions
- [X] Actions can run local scripts
- [X] Log everything that happens
    - [X] Log inputs and actions to Postgres tables
    - [X] Trigger Discord webhooks
- [X] Events go into a queue for processing
- [X] Actions are triggered from a queue

