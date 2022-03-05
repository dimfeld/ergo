import type { WorkerMessage } from './messages';

function sendMessage(name, data) {
  self.postMessage({ name, data });
}

self.onmessage = function handleMessage(ev: MessageEvent<WorkerMessage>) {
  let { name, id, data } = ev.data;

  const ctx = {
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

  switch (name) {
    case 'run_script':
      runScript(ctx, data);
      break;
    default:
      return ctx.reject(new Error(`No handler for message name ${name}`));
  }
};

self.onerror = function (msg, url, line, column, error) {
  sendMessage('error', error);
};

self.onunhandledrejection = (event) => {
  sendMessage('error', event.reason);
};

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

// This looks weird but is the MDN-approved way to get a reference to AsyncFunction.
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

async function runScript(ctx, { script, context, payload }) {
  let actions: { name: string; data: unknown }[] = [];
  const Ergo = {
    getPayload() {
      return payload;
    },
    getContext() {
      return context;
    },
    setContext(ctx: object) {
      context = ctx;
    },
    runAction(actionName: string, actionData: unknown) {
      actions.push({
        name: actionName,
        data: actionData,
      });
    },
  };

  let fn = new AsyncFunction('Ergo', script);
  try {
    await fn(Ergo);
    ctx.resolve({
      context,
      actions,
    });
  } catch (e) {
    ctx.reject(e);
  }
}
