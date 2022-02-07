export interface RunOutputAction {
  name: string;
  payload: object;
}

export interface RunOutput {
  id: number;
  context: object;
  actions: RunOutputAction[];
}
