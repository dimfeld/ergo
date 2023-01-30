import type { AbortError, BundlerWorkerMessage, Result } from './types';
import bundle from './bundle';
import { clearCache } from './packages';

const activeJobs = new Set<number>();

function abortedError(message = 'aborted') {
  let e = new Error(message);
  (e as AbortError).aborted = true;
  return e;
}

function checkActiveJob(jobId: number) {
  if (!activeJobs.has(jobId)) {
    throw abortedError();
  }
}

function cloneableError(e: Error) {
  if (!e) {
    return e;
  }

  return {
    ...e,
    message: e.message,
    stack: e.stack,
  };
}

function cloneableResult(result: Result) {
  if (result.error) {
    return {
      ...result,
      error: cloneableError(result.error),
    };
  } else {
    return result;
  }
}

self.onmessage = async (event: MessageEvent<BundlerWorkerMessage>) => {
  switch (event.data.type) {
    case 'clear_cache':
      clearCache();
      break;
    case 'cancel':
      activeJobs.delete(event.data.data.jobId);
      break;
    case 'bundle': {
      let job = event.data.data;
      let { jobId } = job;
      try {
        activeJobs.add(jobId);
        let checkActive = () => checkActiveJob(jobId);
        let result = await bundle({ ...event.data.data, checkActive });
        if ((result.error as AbortError)?.aborted) {
          return;
        }
        self.postMessage(cloneableResult(result));
      } catch (e: unknown) {
        self.postMessage({ type: 'error', jobId, error: e });
      } finally {
        activeJobs.delete(jobId);
      }
      break;
    }
  }
};
