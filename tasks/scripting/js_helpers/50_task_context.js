Ergo.getContext = function() {
  let context = globalThis.__ergo_context;
  if(typeof context === 'string' && context.length) {
    return eval(`(${context})`);
  }
}

Ergo.setContext = function(context) {
  globalThis.__ergo_context = __ergo_devalue(context);
}
