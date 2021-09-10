import dotenv from 'dotenv';
import preprocess from 'svelte-preprocess';
import * as path from 'path';
import * as url from 'url';
import adapter from '@sveltejs/adapter-static';
import postcssConfig from './postcss.config.cjs';

const dotEnvPath = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '../.env');
dotenv.config({ path: dotEnvPath });

/** @type {import('@sveltejs/kit').Config} */
const config = {
  // Consult https://github.com/sveltejs/svelte-preprocess
  // for more information about preprocessors
  preprocess: [
    preprocess({
      postcss: postcssConfig,
      typescript: {
        compilerOptions: {
          target: 'esnext',
        },
      },
      sourceMap: true,
    }),
  ],

  kit: {
    adapter: adapter({
      fallback: 'index.html',
    }),
    hostHeader: 'X-Forwarded-Host',
    ssr: false,
    vite: () => ({
      // Vite SSR needs this for packages that expose native ESM exports to Node.
      ssr: {
        noExternal: ['sorters'],
      },
      define: {
        'window.ERGO_API_KEY': `'${process.env.API_KEY}'`,
      },
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
