import Worker from './worker?worker';
import {
  workerShell,
  type SandboxHandlers,
  type SandboxWorker,
} from '$lib/sandbox/messages_common';

export type SandboxMessageName = 'init_state' | 'set_node_code';

export interface InitStateArguments {}

export interface DataflowSandboxWorker extends SandboxWorker {
  initState(data: InitStateArguments): Promise<void>;
  initIfNeeded(data: InitStateArguments): Promise<void>;
  setNodeCode(name: string, code: string): Promise<void>;
}

export function sandboxWorker(handlers: SandboxHandlers): DataflowSandboxWorker {
  let intf = workerShell<SandboxMessageName>({ Worker, handlers });

  let needsInit = true;

  async function initState(data: InitStateArguments) {
    return intf.sendMessage<void>('init_state', data);
  }

  return {
    ...intf,
    initState,
    initIfNeeded: async (data: InitStateArguments) => {
      if (!needsInit) {
        return;
      }

      try {
        await initState(data);
      } catch (e) {
        needsInit = true;
        throw e;
      }
    },
    setNodeCode: (name: string, code: string) => intf.sendMessage('set_node_code', { name, code }),
  };
}
