import bundle from '../bundle';
import type { BundleResult } from '../index';
import * as vm from 'vm';

test('Compiles a simple file', async () => {
  let source = `
    let y : number = 5;
    globalThis.x = y;
  `;

  let output = await bundle({
    jobId: 1,
    checkActive: () => true,
    files: {
      'index.ts': source,
    },
  });

  // console.dir(output);
  expect(output.error).toBeFalsy();

  let run = new vm.Script((output as BundleResult).code);
  let ctx: any = {};
  run.runInNewContext(ctx);
  // console.dir(ctx);
  expect(ctx.x).toBe(5);
});

test('Imports packages', async () => {
  // import two packages that have internal functions with the same name to ensure that they get bundled properly.
  let source = `
    import * as sorters from 'sorters';
    import get from 'just-safe-get';
    let list = [
      {a: 5},
      {a: 2},
      {a: 1}
    ];

    list.sort(sorters.sorter((o) => get(o, 'a')));
    globalThis.output = list;
  `;

  let output = await bundle({
    jobId: 2,
    checkActive: () => true,
    files: {
      'index.ts': source,
    },
  });

  // console.dir(output);
  expect(output.error).toBeFalsy();

  let run = new vm.Script((output as BundleResult).code);
  let ctx: any = {};
  run.runInNewContext(ctx);
  // console.dir(ctx);
  expect(ctx.output).toEqual([{ a: 1 }, { a: 2 }, { a: 5 }]);
});
