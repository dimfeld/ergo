import {
  workerShell,
  type SandboxHandlers,
  type SandboxWorker,
} from '$lib/sandbox/messages_common';
import Worker from './worker?worker';

export interface RunOutputAction {
  name: string;
  payload: object;
}

export interface RunOutput {
  id: number;
  context: object;
  actions: RunOutputAction[];
}

export interface RunError {
  id: number;
  error: Error;
}

export interface RunScriptArguments {
  script: string;
  context: object;
  payload: object;
}

export interface ScriptSimulatorWorker extends SandboxWorker {
  runScript(data: RunScriptArguments, timeout?: number): Promise<RunOutput | { error: Error }>;
}

export function sandboxWorker(handlers: SandboxHandlers): ScriptSimulatorWorker {
  let intf = workerShell(Worker, handlers);

  return {
    ...intf,
    runScript(data: RunScriptArguments, timeout?: number) {
      return intf.sendMessage<RunOutput>('run_script', data, timeout);
    },
  };
}
