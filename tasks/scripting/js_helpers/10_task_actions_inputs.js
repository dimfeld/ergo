globalThis.__ergo_actionQueue = [];
Ergo.runAction = function(name, payload) {
  globalThis.__ergo_actionQueue.push({ name, payload });
}

Ergo.getPayload = function() {
  return globalThis.__ergo_inputPayload;
}
