import Worker from './worker?worker';

export interface RunOutputAction {
  name: string;
  payload: object;
}

export interface ConsoleMessage {
  level: string;
  args: unknown[];
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

export interface WorkerMessage {
  name: string;
  data: any;
}

interface Pending {
  reject: (e: Error) => void;
  resolve: (data: any) => void;
}

export interface RunScriptArguments {
  script: string;
  context: object;
  payload: object;
}

export interface SandboxWorker {
  sendMessage(message: string, data: any): Promise<any>;
  /** Terminate and restart the worker. Useful to handle stalled jobs, runaway loops, etc. */
  restart(): void;
  runScript(data: RunScriptArguments, timeout?: number): Promise<RunOutput>;
  destroy(): void;
}

let msgId = 1;

export function sandboxWorker(handlers: Record<string, (data: any) => void>): SandboxWorker {
  const pending = new Map<number, Pending>();
  let worker = new Worker();

  function handleWorkerMessage(evt: MessageEvent<WorkerMessage>) {
    const msg = evt.data;

    if (msg.id) {
      let handler = pending.get(msg.id);
      if (!handler) {
        console.error('Received message for unknown id ' + msg.id);
        return;
      }

      pending.delete(msg.id);

      if (msg.name === 'respond_reject') {
        let { stack, message } = msg.data;

        // Reconstruct the error from the other side.
        let e = new Error(message);
        e.stack = stack;

        handler.reject(e);
      } else if (msg.name === 'respond_resolve') {
        pending.delete(msg.id);
        handler.resolve(msg.data);
      }
    } else {
      handlers[msg.name]?.(msg.data);
    }
  }

  worker.onmessage = handleWorkerMessage;

  function sendMessage<RETVAL>(message: string, data: any) {
    let id = msgId++;

    return new Promise<RETVAL>((resolve, reject) => {
      pending.set(id, { resolve, reject });
      worker.postMessage({ name: message, data, id });
    });
  }

  function destroy() {
    worker?.terminate();
    let terminationError = new Error('Worker terminated');
    for (let val of pending.values()) {
      val.reject(terminationError);
    }
    pending.clear();
  }

  function restart() {
    destroy();
    worker = new Worker();
  }

  return {
    sendMessage,
    restart,
    async runScript(data: RunScriptArguments, timeout?: number) {
      let promise = sendMessage<RunOutput>('run_script', data);
      if (timeout) {
        let result = await Promise.race([
          promise.catch((e) => {
            return {
              error: e as Error,
            };
          }),
          new Promise<'TIMEOUT'>((res) => setTimeout(() => res('TIMEOUT'), timeout)),
        ]);

        if (result === 'TIMEOUT') {
          restart();
          throw new Error('Timed out');
        }

        return result;
      } else {
        return promise;
      }
    },
    destroy,
  };
}
