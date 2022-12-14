globalThis.Deno.core.initializeAsyncOps();
globalThis.__bootstrap.net?.setup(false);
globalThis.Deno.core.setMacrotaskCallback(globalThis.__bootstrap.timers.handleTimerMacrotask);

if(globalThis.__allowTimers !== false) {
  Object.assign(globalThis, globalThis.__bootstrap.timers);
}

if(globalThis.__handleUncaughtPromiseRejections === false) {
  Deno.core.setPromiseRejectCallback(() => {});
}
