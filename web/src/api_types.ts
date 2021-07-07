export type ActionPayloadBuilder = {
  c: {
    [k: string]: ActionInvokeDefDataField;
  };
  t: "FieldMap";
  [k: string]: unknown;
};

export type ActionInvokeDefDataField =
  | {
      c: [string, boolean];
      t: "Input";
      [k: string]: unknown;
    }
  | {
      c: [string, boolean];
      t: "Context";
      [k: string]: unknown;
    }
  | {
      c: true;
      t: "Constant";
      [k: string]: unknown;
    };

export interface ActionInvokeDef {
  data: ActionPayloadBuilder;
  task_action_local_id: string;
  [k: string]: unknown;
}

export type ScriptOrTemplate = {
  c: [string, true][];
  t: "Template";
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
      choices: string[];
      max?: number | null;
      min?: number | null;
      type: "choice";
      [k: string]: unknown;
    };

export interface ActionPayload {
  account_required: boolean;
  account_types?: string[] | null;
  action_category_id: number;
  action_id?: number | null;
  description?: string | null;
  executor_id: string;
  executor_template: ScriptOrTemplate;
  name: string;
  template_fields: {
    [k: string]: TemplateField;
  };
  [k: string]: unknown;
}

export interface TemplateField {
  description?: string | null;
  format: TemplateFieldFormat;
  optional: boolean;
  [k: string]: unknown;
}

export type TransitionTarget = {
  c: string;
  t: "One";
  [k: string]: unknown;
};

export interface EventHandler {
  actions?: ActionInvokeDef[] | null;
  target?: TransitionTarget | null;
  trigger_id: string;
  [k: string]: unknown;
}

export interface Input {
  description?: string | null;
  input_category_id?: number | null;
  input_id: number;
  name: string;
  payload_schema: true;
  [k: string]: unknown;
}

export interface InputPayload {
  description?: string | null;
  input_category_id?: number | null;
  input_id?: number | null;
  name: string;
  payload_schema: true;
  [k: string]: unknown;
}

export interface StateDefinition {
  description?: string | null;
  on: EventHandler[];
  [k: string]: unknown;
}

export interface StateMachine {
  description?: string | null;
  initial: string;
  name: string;
  on?: EventHandler[];
  states: {
    [k: string]: StateDefinition;
  };
  [k: string]: unknown;
}

export interface StateMachineData {
  context: true;
  state: string;
  [k: string]: unknown;
}

export interface TaskInput {
  actions: {
    [k: string]: TaskActionInput;
  };
  description?: string | null;
  enabled: boolean;
  name: string;
  state_machine_config: StateMachine[];
  state_machine_states: StateMachineData[];
  triggers: {
    [k: string]: TaskTriggerInput;
  };
  [k: string]: unknown;
}

export interface TaskActionInput {
  account_id?: number | null;
  action_id: number;
  action_template?: [string, true][] | null;
  name: string;
  [k: string]: unknown;
}

export interface TaskTriggerInput {
  description?: string | null;
  input_id: number;
  name: string;
  [k: string]: unknown;
}

export interface TransitionCondition {
  cond: string;
  target: string;
  [k: string]: unknown;
}