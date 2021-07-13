export type ActionPayloadBuilder = {
  t: "FieldMap";
  c: {
    [k: string]: ActionInvokeDefDataField;
  };
  [k: string]: unknown;
};

export type ActionInvokeDefDataField =
  | {
      t: "Input";
      c: [string, boolean];
      [k: string]: unknown;
    }
  | {
      t: "Context";
      c: [string, boolean];
      [k: string]: unknown;
    }
  | {
      t: "Constant";
      c: true;
      [k: string]: unknown;
    };

export interface ActionInvokeDef {
  task_action_local_id: string;
  data: ActionPayloadBuilder;
  [k: string]: unknown;
}

export type ScriptOrTemplate = {
  t: "Template";
  c: [string, true][];
  [k: string]: unknown;
};

export type TemplateFieldFormat =
  | {
      type: "string";
      [k: string]: unknown;
    }
  | {
      type: "string_array";
      [k: string]: unknown;
    }
  | {
      type: "integer";
      [k: string]: unknown;
    }
  | {
      type: "float";
      [k: string]: unknown;
    }
  | {
      type: "boolean";
      [k: string]: unknown;
    }
  | {
      type: "object";
      [k: string]: unknown;
    }
  | {
      type: "choice";
      choices: string[];
      min?: number | null;
      max?: number | null;
      [k: string]: unknown;
    };

export interface ActionPayload {
  action_id?: number | null;
  action_category_id: number;
  name: string;
  description?: string | null;
  executor_id: string;
  executor_template: ScriptOrTemplate;
  template_fields: {
    [k: string]: TemplateField;
  };
  account_required: boolean;
  account_types?: string[] | null;
  [k: string]: unknown;
}

export interface TemplateField {
  format: TemplateFieldFormat;
  optional: boolean;
  description?: string | null;
  [k: string]: unknown;
}

export type TransitionTarget = {
  t: "One";
  c: string;
  [k: string]: unknown;
};

export interface EventHandler {
  trigger_id: string;
  target?: TransitionTarget | null;
  actions?: ActionInvokeDef[] | null;
  [k: string]: unknown;
}

export interface Input {
  input_id: number;
  input_category_id?: number | null;
  name: string;
  description?: string | null;
  payload_schema: true;
  [k: string]: unknown;
}

export interface InputPayload {
  input_id?: number | null;
  input_category_id?: number | null;
  name: string;
  description?: string | null;
  payload_schema: true;
  [k: string]: unknown;
}

export type InputStatus = "pending" | "success" | "error";

export type ActionStatus = "success" | "pending" | "running" | "error";

export interface InputsLogEntry {
  inputs_log_id: string;
  task_name: string;
  external_task_id: string;
  input_status: InputStatus;
  input_error: true;
  task_trigger_name: string;
  task_trigger_local_id: string;
  timestamp: string;
  actions: InputLogEntryAction[];
  [k: string]: unknown;
}

export interface InputLogEntryAction {
  actions_log_id: string;
  task_action_local_id: string;
  task_action_name: string;
  result: true;
  status: ActionStatus;
  timestamp: string;
  [k: string]: unknown;
}

export interface StateDefinition {
  description?: string | null;
  on: EventHandler[];
  [k: string]: unknown;
}

export interface StateMachine {
  name: string;
  description?: string | null;
  initial: string;
  on?: EventHandler[];
  states: {
    [k: string]: StateDefinition;
  };
  [k: string]: unknown;
}

export interface StateMachineData {
  state: string;
  context: true;
  [k: string]: unknown;
}

export interface TaskDescription {
  id: string;
  name: string;
  description?: string | null;
  enabled: boolean;
  created: string;
  modified: string;
  last_triggered?: string | null;
  successes: number;
  failures: number;
  stats_since: string;
  [k: string]: unknown;
}

export interface TaskInput {
  name: string;
  description?: string | null;
  enabled: boolean;
  state_machine_config: StateMachine[];
  state_machine_states: StateMachineData[];
  actions: {
    [k: string]: TaskActionInput;
  };
  triggers: {
    [k: string]: TaskTriggerInput;
  };
  [k: string]: unknown;
}

export interface TaskActionInput {
  name: string;
  action_id: number;
  account_id?: number | null;
  action_template?: [string, true][] | null;
  [k: string]: unknown;
}

export interface TaskTriggerInput {
  input_id: number;
  name: string;
  description?: string | null;
  [k: string]: unknown;
}

export interface TransitionCondition {
  target: string;
  cond: string;
  [k: string]: unknown;
}