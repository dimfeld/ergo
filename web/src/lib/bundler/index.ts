import type { BundlerWorkerMessage } from './worker';
import { SourceMap } from 'rollup';
import Worker from './worker?worker';

export interface BundleJob {
  files: Record<string, string>;
  production?: boolean;
}

export interface BundleResult {
  jobId: number;
  code: string;
  map?: SourceMap;
  warnings: string[];
  error: null;
}

export type AbortError = Error & { aborted: true };

export interface ErrorResult {
  jobId: number;
  error: Error | AbortError;
}

export type Result = BundleResult | ErrorResult;

interface JobData {
  resolve: (r: Result) => void;
  reject: (e: Error) => void;
}

export class Bundler {
  worker: Worker;
  lastJobId = 0;
  activeJobs = new Map<number, JobData>();

  constructor() {
    this.worker = new Worker();
    this.worker.onmessage = (e) => this._handleMessage(e);
  }

  bundle(job: BundleJob, abort?: AbortSignal): Promise<Result> {
    let jobId = this.lastJobId++;
    if (abort) {
      abort.addEventListener('abort', () => this._cancel(jobId));
    }

    return new Promise<Result>((resolve, reject) => {
      if (abort?.aborted) {
        throw new DOMException('aborted', 'AbortError');
      }

      this.activeJobs.set(jobId, { resolve, reject });
      this._postMessage({
        type: 'bundle',
        data: {
          jobId,
          ...job,
        },
      });
    });
  }

  clearCache() {
    this._postMessage({ type: 'clear_cache' });
  }

  _postMessage(message: BundlerWorkerMessage) {
    this.worker.postMessage(message);
  }

  _handleMessage(e: MessageEvent<Result>) {
    let job = this.activeJobs.get(e.data.jobId);
    if (job) {
      this.activeJobs.delete(e.data.jobId);
      job.resolve(e.data);
    }
  }

  _cancel(jobId: number) {
    let job = this.activeJobs.get(jobId);
    if (job) {
      this.activeJobs.delete(jobId);
      this._postMessage({ type: 'cancel', data: { jobId } });
      job.reject(new DOMException('aborted', 'AbortError'));
    }
  }
}
