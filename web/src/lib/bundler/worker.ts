import { SourceMap } from 'rollup';
import bundle from './bundle';
import { clearCache } from './packages';

export interface JobData {
  jobId: string;
  files: Record<string, string>;
  production?: boolean;
}

export const activeJobs = new Set<string>();

export function abortedError(message = 'aborted') {
  let e = new Error(message);
  (e as AbortError).aborted = true;
  return e;
}

export type AbortError = Error & { aborted: true };

export function checkActiveJob(jobId: string) {
  if (!activeJobs.has(jobId)) {
    throw abortedError();
  }
}

export interface ClearCacheMessage {
  type: 'clear_cache';
}

export interface BundleMessage {
  type: 'bundle';
  data: JobData;
}

export interface CancelMessage {
  type: 'cancel';
  data: { jobId: string };
}

export type BundlerWorkerMessage = ClearCacheMessage | BundleMessage | CancelMessage;

export interface BundleResult {
  jobId: string;
  code: string;
  map?: SourceMap;
  warnings: string[];
  error: null;
}

export interface ErrorResult {
  jobId: string;
  error: Error | AbortError;
}

export type Result = BundleResult | ErrorResult;

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
      let { jobId } = event.data.data;
      try {
        activeJobs.add(jobId);
        let result = await bundle(event.data.data);
        if ((result.error as AbortError)?.aborted) {
          return;
        }

        postMessage(cloneableResult(result));
      } catch (e: unknown) {
        postMessage({ jobId, error: e });
      } finally {
        activeJobs.delete(jobId);
      }
      break;
    }
  }
});
