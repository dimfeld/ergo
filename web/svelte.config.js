import preprocess from 'svelte-preprocess';
import adapter from '@sveltejs/adapter-static';
import postcssConfig from './postcss.config.cjs';
import { vitePreprocess } from '@sveltejs/kit/vite';

const config = {
  preprocess: [
    vitePreprocess({
      style: {
        configFile: './postcss.config.cjs',
      },
    }),
  ],
  // preprocess({
  //   postcss: postcssConfig,
  //   typescript: true,
  //   sourceMap: true,
  // }),
  // ],
  kit: {
    adapter: adapter({
      fallback: 'index.html',
    }),
  },
};

export default config;
