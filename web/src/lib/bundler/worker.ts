import { AbortError, BundleJob, Result } from './index';
import bundle from './bundle';
import { clearCache } from './packages';

export const activeJobs = new Set<number>();

export function abortedError(message = 'aborted') {
  let e = new Error(message);
  (e as AbortError).aborted = true;
  return e;
}

export function checkActiveJob(jobId: number) {
  if (!activeJobs.has(jobId)) {
    throw abortedError();
  }
}

export interface ClearCacheMessage {
  type: 'clear_cache';
}

export interface BundleMessage {
  type: 'bundle';
  data: { jobId: number } & BundleJob;
}

export interface CancelMessage {
  type: 'cancel';
  data: { jobId: number };
}

export type BundlerWorkerMessage = ClearCacheMessage | BundleMessage | CancelMessage;

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

self.addEventListener('message', async (event: MessageEvent<BundlerWorkerMessage>) => {
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
        let result = await bundle(event.data.data);
        if ((result.error as AbortError)?.aborted) {
          return;
        }

        self.postMessage(cloneableResult(result));
      } catch (e: unknown) {
        self.postMessage({ jobId, error: e });
      } finally {
        activeJobs.delete(jobId);
      }
      break;
    }
  }
});
