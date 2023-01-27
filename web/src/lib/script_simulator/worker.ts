import {
  initConsoleHandlers,
  initErrorHandlers,
  initMessageHandler,
  type WorkerMessage,
} from '$lib/sandbox/worker_common';
import type { RunScriptArguments, ScriptSimulatorMessage } from './messages';

initErrorHandlers();
initConsoleHandlers();

// This looks weird but is the MDN-approved way to get a reference to AsyncFunction.
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

async function runScript(msg: WorkerMessage<RunScriptArguments>) {
  let { script, context, payload } = msg.data;
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
    return {
      type: 'success',
      context,
      actions,
    };
  } catch (e) {
    // Return errors from the task code as regular messages since we want to record them in the log.
    return {
      type: 'error',
      error: e,
    };
  }
}

initMessageHandler<ScriptSimulatorMessage>({
  run_script: runScript,
});
