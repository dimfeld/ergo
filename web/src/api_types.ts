export type ActionPayloadBuilder =
  | {
      t: "FieldMap";
      c: {
        [k: string]: ActionInvokeDefDataField;
      };
      [k: string]: unknown;
    }
  | {
      t: "Script";
      c: string;
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
      c: any;
      [k: string]: unknown;
    }
  | {
      t: "Script";
      c: string;
      [k: string]: unknown;
    };

export interface ActionInvokeDef {
  task_action_local_id: string;
  data: ActionPayloadBuilder;
  [k: string]: unknown;
}

export type String = string;

export type ScriptOrTemplate =
  | {
      t: "Template";
      c: [string, true][];
      [k: string]: unknown;
    }
  | {
      t: "Script";
      c: string;
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
  action_id?: String | null;
  action_category_id: String;
  name: string;
  description?: string | null;
  executor_id: string;
  executor_template: ScriptOrTemplate;
  template_fields: TemplateField[];
  account_required: boolean;
  account_types?: string[] | null;
  [k: string]: unknown;
}

export interface TemplateField {
  name: string;
  format: TemplateFieldFormat;
  optional: boolean;
  description?: string | null;
  [k: string]: unknown;
}

export type TransitionTarget =
  | {
      t: "One";
      c: string;
      [k: string]: unknown;
    }
  | {
      t: "Script";
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
  input_id: String;
  input_category_id?: String | null;
  name: string;
  description?: string | null;
  payload_schema: any;
  [k: string]: unknown;
}

export interface InputPayload {
  input_id?: String | null;
  input_category_id?: String | null;
  name: string;
  description?: string | null;
  payload_schema: any;
  [k: string]: unknown;
}

export type InputStatus = "pending" | "success" | "error";

export type ActionStatus = "success" | "pending" | "running" | "error";

export interface InputsLogEntry {
  inputs_log_id: string;
  task_name: string;
  task_id: String;
  input_status: InputStatus;
  input_error: any;
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
  result: any;
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
  context: any;
  [k: string]: unknown;
}

export interface TaskDescription {
  task_id: String;
  name: string;
  description?: string | null;
  alias?: string | null;
  enabled: boolean;
  created: string;
  modified: string;
  last_triggered?: string | null;
  successes: number;
  failures: number;
  stats_since: string;
  [k: string]: unknown;
}

export type TaskConfig = {
  type: "StateMachine";
  data: StateMachine[];
  [k: string]: unknown;
};

export type TaskState = {
  type: "StateMachine";
  data: StateMachineData[];
  [k: string]: unknown;
};

export interface TaskInput {
  name: string;
  description?: string | null;
  alias?: string | null;
  enabled: boolean;
  compiled: TaskConfig;
  source: any;
  state: TaskState;
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
  action_id: String;
  account_id?: String | null;
  action_template?: [string, true][] | null;
  [k: string]: unknown;
}

export interface TaskTriggerInput {
  input_id: String;
  name: string;
  description?: string | null;
  [k: string]: unknown;
}

export interface TransitionCondition {
  target: string;
  cond: string;
  [k: string]: unknown;
}