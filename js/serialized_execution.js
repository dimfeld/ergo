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

ErgoSerialize.wrapAsyncFunction = function wrapAsyncFunction(fn, exitIfUnsaved = false, preserveFn = null, reviveFn = null) {
  return async function(...args) {
    let saved = ErgoSerialize.getResult(exitIfUnsaved, fn.name, args);
    if(saved !== ErgoSerialize.noNewResults) {
      if(saved instanceof Error) {
        throw saved;
      } else {
        return reviveFn ? reviveFn(saved) : saved;
      }
    }

    try {
      let result = await fn(...args);
      let preserved = result;
      if(preserveFn) {
        let p = await preserveFn(result);
        result = p.live;
        preserved = p.preserved;
      }

      ErgoSerialize.saveResult(fn.name, args, preserved);
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
  // TODO Replace Math.random with a version that uses the seed.
  // TODO Use the serialized wall time for Date.
  if(window.fetch) {
    async function preserveFetchResponse(response) {
      // Convert the response into something we can save outside the system. Specifically,
      // without reading the blob its data is stored elsewhere in the V8 runtime and so we
      // need to actually consume it.
      let blob = await response.blob();
      let buffer = await blob.arrayBuffer();
      return {
        preserved: {
          buffer,
          status: response.status,
          statusText: response.statusText,
          headers: response.headers,
        },
        // Create a new response to return right now, so that the caller
        // has a stream to consume.
        live: new Response(blob, response),
      };
    }

    function reviveFetchResponse(response) {
      let { buffer, ...init } = response;
      return new Response(new Blob([buffer]), init);
    }

    window.fetch = ErgoSerialize.wrapAsyncFunction(window.fetch, false, preserveFetchResponse, reviveFetchResponse);
  }
})(globalThis);
