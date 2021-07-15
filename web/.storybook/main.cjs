const preprocess = require('svelte-preprocess');
const postcssConfig = require('../postcss.config.cjs');
module.exports = {
  "stories": [
    "../src/**/*.stories.mdx",
    "../src/**/*.stories.@(js|jsx|ts|tsx|svelte)"
  ],
  "addons": [
    "@storybook/addon-links",
    "@storybook/addon-essentials",
    "@storybook/addon-svelte-csf",
    "storybook-tailwind-dark-mode",
    {
     name: '@storybook/addon-postcss',
     options: {
       postcssLoaderOptions: {
         implementation: require('postcss'),
         postcssOptions: postcssConfig,
       },
     },
   },
  ],
  "svelteOptions": {
    "preprocess": preprocess({
      postcss: postcssConfig,
      typescript: true,
      sourceMap: true
    })
  }
}
