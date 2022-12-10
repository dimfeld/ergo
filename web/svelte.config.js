import preprocess from 'svelte-preprocess';
import adapter from '@sveltejs/adapter-static';
import postcssConfig from './postcss.config.cjs';

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
    adapter: adapter({
      fallback: 'index.html',
    }),
  },
};

export default config;
