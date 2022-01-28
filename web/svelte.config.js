import dotenv from 'dotenv';
import preprocess from 'svelte-preprocess';
import * as path from 'path';
import * as url from 'url';
import adapter from '@sveltejs/adapter-static';
import postcssConfig from './postcss.config.cjs';

let dirname = path.dirname(url.fileURLToPath(import.meta.url));
const dotEnvPath = path.resolve(dirname, '../.env');
dotenv.config({ path: dotEnvPath });

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
  disableDependencyReinclusion: ['svench'],
  kit: {
    adapter: adapter({
      fallback: 'index.html',
    }),
    vite: () => ({
      define: {
        'window.ERGO_API_KEY': `'${process.env.API_KEY}'`,
      },
      ssr: {
        noExternal: ['ergo-wasm', 'sorters'],
      },
      optimizeDeps: {
        exclude: ['rollup'],
      },
      server: {
        fs: {
          allow: ['.', path.resolve('../wasm/pkg')],
        },
        proxy: {
          '/api': `http://localhost:${process.env.BIND_PORT || 6543}`,
        },
      },
    }),
  },
};

export default config;
