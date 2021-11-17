import * as path from 'path';

export default {
  vite: {
    server: {
      host: '0.0.0.0',
    },
    resolve: {
      dedupe: ['svelte'],
      // Since some packages assume that "module" means Node :(
      alias: {
        $lib: path.resolve(process.cwd(), 'src/lib'),
        svelte: path.resolve(process.cwd(), 'node_modules/svelte'),
      },
    },
  },
};
