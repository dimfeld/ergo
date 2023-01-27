import type { MessageReturnType, WorkerMessage } from './messages_common';

export type { WorkerMessage };

function sendMessage(name, data) {
  self.postMessage({ name, data });
}

export interface WorkerContext<Messages> {
  msg: WorkerMessage<Messages>;
  reject(error: Error): void;
  resolve(data: unknown): void;
  resolved(): boolean;
}

export function getMessageContext<Messages>(
  ev: MessageEvent<WorkerMessage<Messages>>
): WorkerContext<Messages> {
  let { id } = ev.data;

  let resolved = false;
  return {
    msg: ev.data,
    resolved: () => resolved,
    resolve: (data: MessageReturnType<Messages, keyof Messages>) => {
      resolved = true;
      self.postMessage({ id, name: 'respond_resolve', data });
    },
    reject: (error) => {
      resolved = true;
      let data = {
        ...error,
        message: error.message,
        stack: error.stack,
      };
      self.postMessage({ id, name: 'respond_reject', data });
    },
  };
}

export function initMessageHandler<Messages>(handlers: Required<Messages>) {
  self.onmessage = async (ev: MessageEvent<WorkerMessage<Messages>>) => {
    const ctx = getMessageContext(ev);

    const handler = handlers[ctx.msg.name];
    if (!handler) {
      // TODO See why Typescript things ctx.msg.name might be a symbol.
      ctx.reject(new Error(`No handler for ${ctx.msg.name as string}`));
      return;
    }

    try {
      let result = await handler(ctx.msg);
      if (!ctx.resolved()) {
        ctx.resolve(result);
      }
    } catch (e) {
      if (!ctx.resolved()) {
        ctx.reject(e);
      }
    }
  };
}

export function initErrorHandlers() {
  self.onerror = function (msg, url, line, column, error) {
    sendMessage('error', error);
  };

  self.onunhandledrejection = (event: PromiseRejectionEvent) => {
    sendMessage('error', event.reason);
  };
}

export function initConsoleHandlers() {
  // Send console messages to the outer shell
  for (let type of ['info', 'dir', 'warn', 'log', 'error']) {
    const orig = console[type];
    console[type] = (...args: unknown[]) => {
      try {
        sendMessage('console', { level: type, args });
      } catch (e) {
        sendMessage('console', { level: type, args: JSON.stringify(args) });
      }

      orig(...args);
    };
  }
}
