export interface ConsoleMessage {
  level: string;
  args: unknown[];
}

// export type GenericMessageHandler = Record<string, (x: WorkerMessage<any>) => Promise<unknown>>;
export type MessageParameter<
  Messages,
  MessageName extends keyof Messages
> = Messages[MessageName] extends (x: WorkerMessage<unknown>) => Promise<unknown> | unknown
  ? Parameters<Messages[MessageName]>[0]['data']
  : never;
export type MessageReturnType<
  Messages,
  MessageName extends keyof Messages
> = Messages[MessageName] extends (x: WorkerMessage<unknown>) => Promise<unknown> | unknown
  ? Awaited<ReturnType<Messages[MessageName]>>
  : never;

export interface WorkerMessage<Payload> {
  id?: number;
  name: string;
  data: Payload;
}

export interface TypedWorkerMessage<Messages, Message extends keyof Messages = keyof Messages> {
  id?: number;
  name: keyof Messages;
  data: MessageParameter<Messages, Message>;
}

interface Pending {
  reject: (e: Error) => void;
  resolve: (data: unknown) => void;
}

export interface SandboxWorker<Messages> {
  sendMessage<MESSAGE extends keyof Messages>(
    message: MESSAGE,
    data: MessageParameter<Messages, MESSAGE>,
    timeout?: number
  ): Promise<MessageReturnType<Messages, MESSAGE>>;
  /** Terminate and restart the worker. Useful to handle stalled jobs, runaway loops, etc. */
  restart(): void;
  destroy(): void;
}

export type SandboxHandlers = Record<string, (data: unknown) => void>;

let msgId = 1;

export interface WorkerShellArgs {
  Worker: new () => Worker;
  handlers: SandboxHandlers;
  onRestart?: () => void;
}

export function workerShell<Messages>({
  Worker: WorkerFn,
  handlers,
  onRestart,
}: WorkerShellArgs): SandboxWorker<Messages> {
  const pending = new Map<number, Pending>();
  let worker = new WorkerFn();

  function handleWorkerMessage(evt: MessageEvent<WorkerMessage<unknown>>) {
    const msg = evt.data;

    if (msg.id) {
      let handler = pending.get(msg.id);
      if (!handler) {
        console.error('Received message for unknown id ' + msg.id);
        return;
      }

      pending.delete(msg.id);

      if (msg.name === 'respond_reject') {
        let { stack, message } = msg.data as Error;

        // Reconstruct the error from the other side.
        let e = new Error(message);
        e.stack = stack;

        handler.reject(e);
      } else if (msg.name === 'respond_resolve') {
        pending.delete(msg.id);
        handler.resolve(msg.data);
      }
    } else {
      handlers[msg.name as string]?.(msg.data);
    }
  }

  worker.onmessage = handleWorkerMessage;

  async function sendMessage<MESSAGE extends keyof Messages>(
    message: MESSAGE,
    data: MessageParameter<Messages, MESSAGE>,
    timeout?: number
  ): Promise<MessageReturnType<Messages, MESSAGE>> {
    let id = msgId++;

    let promise = new Promise<MessageReturnType<Messages, MESSAGE>>((resolve, reject) => {
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
