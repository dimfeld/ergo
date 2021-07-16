import preprocess from 'svelte-preprocess';
import * as path from 'path';
import adapter from '@sveltejs/adapter-vercel';
import postcssConfig from './postcss.config.cjs';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  // Consult https://github.com/sveltejs/svelte-preprocess
  // for more information about preprocessors
  preprocess: [
    preprocess({
      postcss: postcssConfig,
      typescript: true,
      sourceMap: true,
    }),
  ],

  kit: {
    adapter: adapter(),
    hostHeader: 'X-Forwarded-Host',
    ssr: false,
    vite: () => ({
      server: {
        proxy: {
          '/api': `http://localhost:${process.env.BIND_PORT || 6543}`,
        },
      },
      resolve: {
        dedupe: ['svelte'],
        alias: {
          '^': path.resolve(process.cwd(), 'src'),
          svelte: path.resolve(process.cwd(), 'node_modules/svelte'),
        },
      },
    }),
  },
};

export default config;
