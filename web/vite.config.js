import { sveltekit } from '@sveltejs/kit/vite';
import dotenv from 'dotenv';
import * as path from 'path';
import * as url from 'url';

let dirname = path.dirname(url.fileURLToPath(import.meta.url));
const dotEnvPath = path.resolve(dirname, '../.env');
dotenv.config({ path: dotEnvPath });

/** @type {import('vite').UserConfig} */
const config = {
  plugins: [
    sveltekit({
      vitePlugin: {
        disableDependencyReinclusion: ['svench'],
      },
    }),
  ],
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
};

export default config;
