export interface ConsoleMessage {
  level: string;
  args: unknown[];
}

export interface WorkerMessage<MessageName extends string = string, T = any> {
  id?: number;
  name: MessageName;
  data: T;
}

interface Pending {
  reject: (e: Error) => void;
  resolve: (data: any) => void;
}

export interface SandboxWorker<MessageName extends string = string> {
  sendMessage<RETVAL>(message: MessageName, data: any, timeout?: number): Promise<RETVAL>;
  /** Terminate and restart the worker. Useful to handle stalled jobs, runaway loops, etc. */
  restart(): void;
  destroy(): void;
}

export type SandboxHandlers = Record<string, (data: any) => void>;

let msgId = 1;

export interface WorkerShellArgs {
  Worker: new () => Worker;
  handlers: SandboxHandlers;
  onRestart?: () => void;
}

export function workerShell<MessageName extends string = string>({
  Worker: WorkerFn,
  handlers,
  onRestart,
}: WorkerShellArgs): SandboxWorker<MessageName> {
  const pending = new Map<number, Pending>();
  let worker = new WorkerFn();

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

  async function sendMessage<RETVAL>(message: MessageName, data: any, timeout?: number) {
    let id = msgId++;

    let promise = new Promise<RETVAL>((resolve, reject) => {
      pending.set(id, { resolve, reject });
      worker.postMessage({ name: message, data, id });
    });

    if (timeout) {
      let result = await Promise.race([
        promise,
        new Promise<'TIMEOUT'>((res) => setTimeout(() => res('TIMEOUT'), timeout)),
      ]);

      if (result === 'TIMEOUT') {
        restart();
        let e = new Error('Timed out');
        e.name = 'TimeoutError';
        throw e;
      }

      return result;
    } else {
      return promise;
    }
  }

  function destroy() {
    worker?.terminate();
    let terminationError = new Error('Worker terminated');
    terminationError.name = 'WorkerTerminated';
    for (let val of pending.values()) {
      val.reject(terminationError);
    }
    pending.clear();
  }

  function restart() {
    destroy();
    worker = new WorkerFn();
    onRestart?.();
  }

  return {
    sendMessage,
    restart,
    destroy,
  };
}
