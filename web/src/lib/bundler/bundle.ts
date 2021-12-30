import * as rollup from 'rollup';
import ts from 'typescript';
import path from 'path-browserify';
import resolvePackages from './packages';
import { BundleJob, Result } from './index';

interface VirtualOptions {
  checkActive: () => void;
  files?: Record<string, string>;
  modules?: Record<string, string>;
}

const suffixes = ['', '.js', '.ts'];
const VIRTUAL_PREFIX = '\0virtual';

function virtual({ checkActive, files = {}, modules = {} }: VirtualOptions): rollup.Plugin {
  const resolvedIds = new Map([
    ...(Object.entries(files).map(([id, contents]) => {
      return [path.resolve('/', id), contents];
    }) as [string, string][]),
    ...Object.entries(modules),
  ]);

  return {
    name: 'virtual',
    resolveId(source, importer) {
      checkActive();
      const realImporter = importer?.startsWith(VIRTUAL_PREFIX)
        ? importer.slice(VIRTUAL_PREFIX.length)
        : importer;
      // Prefix with root directory since we won't have a real CWD in the browser.
      const importerDir = realImporter ? '/' + path.dirname(realImporter) : '/';
      const resolved = path.resolve(importerDir, source);

      // Only try the suffixes if this path doesn't have one already.
      let thisSuffixes = path.extname(resolved) ? [''] : suffixes;

      for (let suffix of thisSuffixes) {
        let full = resolved + suffix;
        if (resolvedIds.has(full)) {
          return VIRTUAL_PREFIX + full;
        }
      }

      return null;
    },
    load(id) {
      checkActive();
      if (id.startsWith(VIRTUAL_PREFIX)) {
        let p = id.slice(VIRTUAL_PREFIX.length);
        return resolvedIds.get(p);
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
      if (replaced !== code) {
        return { code: replaced };
      }

      return null;
    },
  };
}

export default async function bundle(
  job: BundleJob & { jobId: number; checkActive: () => void }
): Promise<Result> {
  let input = 'index.ts' in job.files ? 'index.ts' : Object.keys(job.files)[0];

  let warnings: string[] = [];
  let bundler = await rollup.rollup({
    input: '/' + input,
    plugins: [
      virtual({ checkActive: job.checkActive, files: job.files }),
      resolvePackages(job.checkActive),
      replace(JSON.stringify(job.production ? 'production' : 'development')),
      {
        name: 'typescript',
        transform(code, id) {
          let result = ts.transpileModule(code, {
            moduleName: id,
            compilerOptions: {
              sourceMap: true,
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
        name: job.name ?? input,
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
