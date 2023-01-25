import type { WorkerMessage } from './messages_common';

export type { WorkerMessage };

function sendMessage(name, data) {
  self.postMessage({ name, data });
}

export interface WorkerContext<MessageName extends string, T> {
  msg: WorkerMessage<MessageName, T>;
  reject(error: Error): void;
  resolve(data: any): void;
}

export function getMessageContext<MessageName extends string = string, T = any>(
  ev: MessageEvent<WorkerMessage<MessageName, T>>
): WorkerContext<MessageName, T> {
  let { id } = ev.data;

  return {
    msg: ev.data,
    resolve: (data) => self.postMessage({ id, name: 'respond_resolve', data }),
    reject: (error) => {
      let data = {
        ...error,
        message: error.message,
        stack: error.stack,
      };
      self.postMessage({ id, name: 'respond_reject', data });
    },
  };
}

export function initErrorHandlers() {
  self.onerror = function (msg, url, line, column, error) {
    sendMessage('error', error);
  };

  self.onunhandledrejection = (event) => {
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
