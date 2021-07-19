const path = require('path');
const preprocess = require('svelte-preprocess');
const postcssConfig = require('../postcss.config.cjs');
module.exports = {
  core: {
    builder: 'webpack5',
  },
  stories: ['../src/**/*.stories.mdx', '../src/**/*.stories.@(js|jsx|ts|tsx|svelte)'].map((d) => path.join(__dirname, d)),
  addons: [
    '@storybook/addon-links',
    '@storybook/addon-essentials',
    '@storybook/addon-svelte-csf',
    'storybook-tailwind-dark-mode',
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
  svelteOptions: {
    preprocess: preprocess({
      postcss: postcssConfig,
      typescript: true,
      sourceMap: true,
    }),
  },
  webpackFinal: async (config) => {
    config.resolve = {
      ...config.resolve,
      alias: {
        ...config.resolve.alias,
        svelte: path.resolve(__dirname, "..", "node_modules", "svelte"),
      },
      mainFields: ["svelte", "browser", "module", "main"],
    };
    config.module.rules.push({
      resolve: {
        fullySpecified: false,
        extensions: ['.js', '.ts']
      },
    });
    return config;
  },
};
