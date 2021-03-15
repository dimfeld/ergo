Ergo is an IFTTT/Zapier style application that runs tasks based on events.

This project is in very early stages right now with no current plans for proper publishing (yet). I'm doing this
mostly for fun and learning.

# Considered use cases

- Scrape web sources and send emails based on some content in them
- Receive a URL, run youtube-dl and save the video locally
- Periodically download Roam database
- Some sort of Twitter list archiving?
- Scrape websites and populate a local database
- Fetch filled brokerage orders

# Anticipated Features

- [ ] Events
  - [ ] from POST to an endpoint
  - [ ] some events are sent based on some periodic check that triggers when it sees a condition
  - [ ] trigger events unconditionally on a schedule
- [ ] Actions
  - [ ] Spawn docker containers (or Nomad jobs?)
  - [ ] Query HTTP endpoints
  - [ ] Run a script
  - [ ] Run some other command
  - [ ] Embedded scripting for actions
  - [ ] Return data for use by the next task in the chain
- [ ] Data Schemas
  - [ ] Each action can specify the types of data that it accepts
  - [ ] Duck typing for events
- [ ] Transformers will take data from an event, modify it somehow, and pass it on
  - [ ] Embed [rhai](https://rhai.rs/) or or maybe JavaScript to write transformers
  - [ ] Optional schema input
  - [ ] Optional type checking on transformer return value
- [ ] Reducers are transformers that keep some persistent state between calls
- [ ] Tasks
  - [ ] Trigger a task based on events
  - [ ] Run actions
  - [ ] Run transformers between the events
  - [ ] Sequences - Actions can be chained together and optionally pass information between them (file locations, etc.)
- [ ] Templates for events, actions, and transformers
- [ ] Extensive logging of events, actions, etc.

# Roadmap

## 0.1

- [ ] Events can be triggered by REST endpoint
- [ ] Tasks consists of one event that triggers one or more actions
- [ ] Actions can run local scripts
- [ ] Simple Web UI
- [ ] Log everything that happens

## 0.2

- Events go into a queue for processing
- Actions are triggered from a queue
- Ability to run Nomad jobs as actions?
