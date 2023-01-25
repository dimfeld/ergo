import type { SourceMap } from 'rollup';

export interface BundleJob {
  name?: string;
  files: Record<string, string>;
  /** Output format. Defaults to ESM */
  format?: 'es' | 'iife';
  production?: boolean;
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

export interface BundleResult {
  type: 'result';
  jobId: number;
  code: string;
  map?: SourceMap;
  warnings: string[];
  error: null;
}

export type AbortError = Error & { aborted: true };

export interface ErrorResult {
  type: 'error';
  jobId: number;
  error: Error | AbortError;
}

export type Result = BundleResult | ErrorResult;

export interface WorkerLog {
  type: 'log';
  data: any[];
}

export interface JobData {
  resolve: (r: Result) => void;
  reject: (e: Error) => void;
}
