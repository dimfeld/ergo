import {
  workerShell,
  type SandboxHandlers,
  type SandboxWorker,
  type WorkerMessage,
} from '$lib/sandbox/messages_common';
import Worker from './worker?worker';

export type { ConsoleMessage } from '$lib/sandbox/messages_common';

export interface RunOutputAction {
  name: string;
  payload: object;
}

export interface RunOutput {
  type: 'success';
  context: object;
  actions: RunOutputAction[];
}

export interface RunError {
  id: number;
  type: 'error';
  error: Error;
}

export interface RunScriptArguments {
  script: string;
  context: object;
  payload: object;
}

export interface ScriptSimulatorMessage {
  run_script: (msg: WorkerMessage<RunScriptArguments>) => Promise<RunOutput | RunError>;
}

export interface ScriptSimulatorWorker extends SandboxWorker<ScriptSimulatorMessage> {
  runScript(data: RunScriptArguments, timeout?: number): Promise<RunOutput | RunError>;
}

export function sandboxWorker(handlers: SandboxHandlers): ScriptSimulatorWorker {
  let intf = workerShell<ScriptSimulatorMessage>({ Worker, handlers });

  return {
    ...intf,
    runScript(data: RunScriptArguments, timeout?: number): Promise<RunOutput | RunError> {
      return intf.sendMessage('run_script', data, timeout);
    },
  };
}
