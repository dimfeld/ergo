import preprocess from 'svelte-preprocess';
import * as path from 'path';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  // Consult https://github.com/sveltejs/svelte-preprocess
  // for more information about preprocessors
  preprocess: [
    preprocess({
      postcss: true,
      typescript: true,
      sourceMap: true,
    }),
  ],

  kit: {
    ssr: false,
    vite: () => ({
      resolve: {
        alias: {
          '^': path.resolve(process.cwd(), 'src'),
        },
      },
    }),
  },
};

export default config;
