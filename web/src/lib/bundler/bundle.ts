import * as rollup from 'rollup';
import * as ts from 'typescript';
import * as path from 'path-browserify';
import MagicString from 'magic-string';
import resolvePackages from './packages';
import { BundleJob, Result } from './index';
import { checkActiveJob } from './worker';

interface VirtualOptions {
  jobId: number;
  files?: Record<string, string>;
  modules?: Record<string, string>;
}

const suffixes = ['', '.js', '.ts'];
const VIRTUAL_PREFIX = '\0virtual';

function virtual({ jobId, files = {}, modules = {} }: VirtualOptions): rollup.Plugin {
  const resolvedIds = new Map([
    ...(Object.entries(files).map(([id, contents]) => {
      return [path.resolve('/', id), contents];
    }) as [string, string][]),
    ...Object.entries(modules),
  ]);

  return {
    name: 'virtual',
    resolveId(source, importer) {
      checkActiveJob(jobId);
      const realImporter = importer?.startsWith(VIRTUAL_PREFIX)
        ? importer.slice(VIRTUAL_PREFIX.length)
        : importer;
      // Prefix with root directory since we won't have a real CWD in the browser.
      const importerDir = realImporter ? '/' + path.dirname(realImporter) : null;

      for (let suffix in suffixes) {
        let full = source + suffix;
        if (resolvedIds.has(full)) {
          return VIRTUAL_PREFIX + full;
        }

        if (importerDir) {
          const resolved = path.resolve(importerDir, full);
          if (resolvedIds.has(resolved)) {
            return VIRTUAL_PREFIX + resolved;
          }
        }
      }

      return null;
    },
    load(id) {
      if (id.startsWith(VIRTUAL_PREFIX)) {
        let p = id.slice(VIRTUAL_PREFIX.length);
        return files[p] ?? resolvedIds.get(p);
      }

      return null;
    },
  };
}

function replace(env: string): rollup.Plugin {
  return {
    name: 'replace',
    transform(code) {
      let replaced = code.replaceAll('process.env.NODE_ENV', env);
      if (replaced) {
        return { code: replaced };
      }

      return null;
    },
  };
}

export default async function bundle(job: BundleJob & { jobId: number }): Promise<Result> {
  let input = 'index.ts' in job.files ? 'index.ts' : Object.keys(job.files)[0];

  let warnings: string[] = [];
  let bundler = await rollup.rollup({
    input,
    plugins: [
      virtual({ jobId: job.jobId, files: job.files }),
      resolvePackages(job.jobId),
      replace(JSON.stringify(job.production ? 'production' : 'development')),
      {
        name: 'typescript',
        transform(code, id) {
          let result = ts.transpileModule(code, {
            moduleName: id,
            compilerOptions: {
              module: ts.ModuleKind.ESNext,
              target: ts.ScriptTarget.ESNext,
              lib: ['esnext'],
            },
          });

          return {
            code: result.outputText,
            map: result.sourceMapText,
          };
        },
      },
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
