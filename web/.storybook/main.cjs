const preprocess = require('svelte-preprocess');
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
       },
     },
   },
  ],
  "svelteOptions": {
    "preprocess": preprocess({
      postcss: true,
      typescript: true,
      sourceMap: true
    })
  }
}
