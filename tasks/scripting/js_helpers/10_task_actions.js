globalThis.__ergo_actionQueue = [];
Ergo.runAction = function(name, payload) {
  globalThis.__ergo_actionQueue.push({ name, payload });
}
