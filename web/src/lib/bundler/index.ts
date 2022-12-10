import BundleWorker from './worker?worker';
import type { BundleJob, BundlerWorkerMessage, JobData, Result } from './types';

export type { BundleJob, Result };

let worker: Worker | undefined;
let clients = new Set<Bundler>();
let lastJobId = 0;

export class Bundler {
  activeJobs = new Map<number, JobData>();
  handler;

  constructor() {
    clients.add(this);
    if (!worker) {
      worker = new BundleWorker();
      worker.onmessageerror = (e) => console.error('worker messageerror', e);
      worker.onerror = (e) => console.error('worker error', e);
    }

    // Save the handler reference so we can remove it later.
    this.handler = (e: MessageEvent<Result>) => this._handleMessage(e);
    worker.addEventListener('message', this.handler);
  }

  bundle(job: BundleJob, abort?: AbortSignal): Promise<Result> {
    let jobId = lastJobId++;
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

  destroy() {
    clients.delete(this);
    this.activeJobs.clear();
    worker?.removeEventListener('message', this.handler);
    if (!clients.size && worker) {
      worker.terminate();
      worker = undefined;
    }
  }

  _postMessage(message: BundlerWorkerMessage) {
    worker!.postMessage(message);
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
