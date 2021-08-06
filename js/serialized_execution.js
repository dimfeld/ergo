ErgoSerialize.wrapSyncFunction = function wrapSyncFunction(fn, exitIfUnsaved = false) {
  return function(...args) {
    let saved = ErgoSerialize.getResult(exitIfUnsaved, fn.name, args);
    if(saved !== ErgoSerialize.noNewResults) {
      if(saved instanceof Error) {
        throw saved;
      } else {
        return saved;
      }
    }

    try {
      let result = fn(...args);
      ErgoSerialize.saveResult(fn.name, args, result);
      return result;
    } catch(e) {
      ErgoSerialize.saveResult(fn.name, args, e);
      throw e;
    }
  };
}

ErgoSerialize.wrapAsyncFunction = function wrapAsyncFunction(fn, exitIfUnsaved = false) {
  return async function(...args) {
    let saved = ErgoSerialize.getResult(exitIfUnsaved, fn.name, args);
    if(saved !== ErgoSerialize.noNewResults) {
      if(saved instanceof Error) {
        throw saved;
      } else {
        return saved;
      }
    }

    try {
      let result = await fn(...args);
      ErgoSerialize.saveResult(fn.name, args, result);
      return result;
    } catch(e) {
      ErgoSerialize.saveResult(fn.name, args, e);
      throw e;
    }
  };
}

ErgoSerialize.externalAction = function(name) {
  return function(...args) {
    return ErgoSerialize.getResult(true, name, args);
  };
};

(function installSerializedExecution(window) {
  if(window.fetch) {
    let origFetch = window.fetch;
    let fetch = async function serializableFetch(...args) {
      // To allow us to serialize the response, we need to read the whole blob now.
      let response = await origFetch(...args);
      let blob = await response.blob();
      return new Response(blob, response);
    };

    window.fetch = wrapAsyncFunction(fetch);
  }
})(globalThis);
