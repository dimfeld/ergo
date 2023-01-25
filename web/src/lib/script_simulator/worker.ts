import {
  getMessageContext,
  initConsoleHandlers,
  initErrorHandlers,
  type WorkerContext,
  type WorkerMessage,
} from '$lib/sandbox/worker_common';
import type { RunScriptArguments } from './messages';

initErrorHandlers();
initConsoleHandlers();

self.onmessage = function handleMessage(ev: MessageEvent<WorkerMessage>) {
  const ctx = getMessageContext(ev);

  switch (ctx.msg.name) {
    case 'run_script':
      runScript(ctx);
      break;
    default:
      return ctx.reject(new Error(`No handler for message name ${name}`));
  }
};

// This looks weird but is the MDN-approved way to get a reference to AsyncFunction.
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

async function runScript(ctx: WorkerContext<string, RunScriptArguments>) {
  let { script, context, payload } = ctx.msg.data;
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
      type: 'success',
      context,
      actions,
    });
  } catch (e) {
    // Return errors from the task code as regular messages since we want to record them in the log.
    ctx.resolve({
      type: 'error',
      error: e,
    });
  }
}
