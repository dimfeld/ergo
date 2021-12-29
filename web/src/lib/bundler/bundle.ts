import * as rollup from 'rollup';
import typescript from '@rollup/plugin-typescript';
import virtual from '@rollup/plugin-virtual';
import replace from '@rollup/plugin-replace';
import resolvePackages from './packages';
import { BundleJob, Result } from './index';

export default async function bundle(job: BundleJob & { jobId: number }): Promise<Result> {
  let input = 'index.ts' in job.files ? 'index.ts' : Object.keys(job.files)[0];

  let files = Object.fromEntries(
    Object.entries(job.files).map(([path, file]) => {
      let outputPath = path === input ? path : './' + path;
      return [outputPath, file];
    })
  );

  let warnings: string[] = [];
  let bundler = await rollup.rollup({
    input,
    plugins: [
      virtual(files),
      resolvePackages(job.jobId),
      replace({
        // Some packages assume this exists even if you aren't in Node
        'process.env.NODE_ENV': JSON.stringify(job.production ? 'production' : 'development'),
      }),
      typescript({
        lib: ['esnext'],
        target: 'esnext',
        tsconfig: false,
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
