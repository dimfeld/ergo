import {
  getMessageContext,
  initConsoleHandlers,
  initErrorHandlers,
  type WorkerContext,
  type WorkerMessage,
} from '$lib/sandbox/worker_common';
import type { SandboxMessageName } from './messages';

initErrorHandlers();
initConsoleHandlers();

self.onmessage = function handleMessage(ev: MessageEvent<WorkerMessage<SandboxMessageName>>) {
  const ctx = getMessageContext(ev);

  switch (ctx.msg.name) {
    case 'init_state':
      break;
    case 'set_node_code':
      break;
    default:
      return ctx.reject(new Error(`No handler for message name ${name}`));
  }
};

// This looks weird but is the MDN-approved way to get a reference to AsyncFunction.
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
