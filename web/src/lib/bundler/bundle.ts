import * as rollup from 'rollup';
import typescript from '@rollup/plugin-typescript';
import virtual from '@rollup/plugin-virtual';
import replace from '@rollup/plugin-replace';
import resolvePackages from './packages';
import { BundleJob, Result } from './worker';

export default async function bundle(job: BundleJob): Promise<Result> {
  let input = 'index.ts' in job.files ? 'index.ts' : Object.keys(job.files)[0];

  let files = Object.fromEntries(
    Object.entries(job.files).map(([path, file]) => {
      return ['./' + path, file];
    })
  );

  let warnings: string[] = [];
  let bundler = await rollup.rollup({
    input,
    plugins: [
      virtual(files),
      resolvePackages(job.jobId),
      typescript(),
      replace({
        'process.env.NODE_ENV': JSON.stringify(job.production ? 'production' : 'development'),
      }),
    ],
    onwarn: (w) => warnings.push(w.message),
  });

  try {
    let result = (
      await bundler.generate({
        format: 'iife',
        name: input,
        sourcemap: true,
      })
    ).output[0];

    return {
      jobId: job.jobId,
      code: result.code,
      map: result.map,
      warnings,
      error: null,
    };
  } catch (e) {
    return {
      jobId: job.jobId,
      error: e as Error,
    };
  }
}
