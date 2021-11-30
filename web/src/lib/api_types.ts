export type String = string;

export type ScriptOrTemplate =
  | {
      t: "Template";
      c: [string, true][];
    }
  | {
      t: "Script";
      c: string;
    };

export type TemplateFieldFormat =
  | {
      type: "string";
    }
  | {
      type: "string_array";
    }
  | {
      type: "integer";
    }
  | {
      type: "float";
    }
  | {
      type: "boolean";
    }
  | {
      type: "object";
    }
  | {
      type: "choice";
      choices: string[];
      min?: number | null;
      max?: number | null;
    };

export type TemplateFields = TemplateField[];

export interface Action {
  action_id: String;
  action_category_id: String;
  name: string;
  description?: string | null;
  executor_id: string;
  executor_template: ScriptOrTemplate;
  template_fields: TemplateFields;
  timeout?: number | null;
  /**
   * A script that processes the executor's JSON result. The result is exposed in the variable `result` and the action's payload is exposed as `payload`. The value returned will replace the executor's return value, or an error can be thrown to mark the action as failed.
   */
  postprocess_script?: string | null;
  account_required: boolean;
  account_types?: string[];
}

export interface TemplateField {
  name: string;
  format: TemplateFieldFormat;
  optional: boolean;
  description?: string | null;
}

export type ActionPayloadBuilder =
  | {
      t: "FieldMap";
      c: {
        [k: string]: ActionInvokeDefDataField;
      };
    }
  | {
      t: "Script";
      c: string;
    };

export type ActionInvokeDefDataField =
  | {
      t: "Input";
      c: [string, boolean];
    }
  | {
      t: "Context";
      c: [string, boolean];
    }
  | {
      t: "Constant";
      c: any;
    }
  | {
      t: "Script";
      c: string;
    };

export interface ActionInvokeDef {
  task_action_local_id: string;
  data: ActionPayloadBuilder;
}

export type TransitionTarget =
  | {
      t: "One";
      c: string;
    }
  | {
      t: "Script";
      c: string;
    };

export interface EventHandler {
  trigger_id: string;
  target?: TransitionTarget | null;
  actions?: ActionInvokeDef[] | null;
}

export interface ExecutorInfo {
  name: string;
  template_fields: TemplateFields;
}

export interface Input {
  input_id: String;
  input_category_id?: String | null;
  name: string;
  description?: string | null;
  payload_schema: any;
}

export interface InputPayload {
  input_category_id?: String | null;
  name: string;
  description?: string | null;
  payload_schema: any;
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
}

export interface InputLogEntryAction {
  actions_log_id: string;
  task_action_local_id: string;
  task_action_name: string;
  result: any;
  status: ActionStatus;
  timestamp: string;
}

export interface StateDefinition {
  description?: string | null;
  on: EventHandler[];
}

export interface StateMachine {
  name: string;
  description?: string | null;
  initial: string;
  on?: EventHandler[];
  states: {
    [k: string]: StateDefinition;
  };
}

export interface StateMachineData {
  state: string;
  context: any;
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
}

export type TaskConfig =
  | {
      type: "StateMachine";
      data: StateMachine[];
    }
  | {
      type: "Js";
      data: TaskJsConfig;
    };

export type TaskState =
  | {
      type: "StateMachine";
      data: StateMachineData[];
    }
  | {
      type: "Js";
      data: TaskJsState;
    };

export type PeriodicSchedule = {
  type: "Cron";
  data: string;
};

export interface TaskInput {
  name: string;
  description?: string | null;
  alias?: string | null;
  enabled: boolean;
  compiled: TaskConfig;
  source: any;
  state?: TaskState | null;
  actions: {
    [k: string]: TaskActionInput;
  };
  triggers: {
    [k: string]: TaskTriggerInput;
  };
}

export interface TaskJsConfig {
  timeout?: number | null;
  script: string;
  /**
   * The source map for the compiled script
   */
  map?: string;
}

export interface TaskJsState {
  context: string;
}

export interface TaskActionInput {
  name: string;
  action_id: String;
  account_id?: String | null;
  action_template?: [string, true][] | null;
}

export interface TaskTriggerInput {
  input_id: String;
  name: string;
  description?: string | null;
  periodic?: PeriodicTaskTriggerInput[] | null;
}

export interface PeriodicTaskTriggerInput {
  name?: string | null;
  schedule: PeriodicSchedule;
  payload: any;
  enabled: boolean;
}

export interface TaskResult {
  task_id: String;
  name: string;
  description?: string | null;
  alias?: string | null;
  enabled: boolean;
  task_template_version: number;
  compiled: TaskConfig;
  source: any;
  state: TaskState;
  created: string;
  modified: string;
  actions: {
    [k: string]: TaskAction;
  };
  triggers: {
    [k: string]: TaskTrigger;
  };
}

export interface TaskAction {
  action_id: String;
  task_local_id: string;
  task_id: String;
  account_id?: String | null;
  name: string;
  action_template?: [string, true][] | null;
}

export interface TaskTrigger {
  task_trigger_id: String;
  task_id: String;
  input_id: String;
  name: string;
  description?: string | null;
  last_payload?: string | null;
  periodic?: PeriodicTaskTrigger[] | null;
}

export interface PeriodicTaskTrigger {
  periodic_trigger_id: String;
  name?: string | null;
  schedule: PeriodicSchedule;
  payload: any;
  enabled: boolean;
}

export interface TransitionCondition {
  target: string;
  cond: string;
}