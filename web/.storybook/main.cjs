const path = require('path');

module.exports = {
  stories: ['../src/**/*.mdx', '../src/**/*.stories.@(js|jsx|svelte)'],
  addons: [
    '@storybook/addon-links',
    '@storybook/addon-essentials',
    '@storybook/addon-interactions',
    '@storybook/addon-svelte-csf',
    'storybook-dark-mode',
  ],
  framework: {
    name: '@storybook/sveltekit',
    options: {},
  },
  docs: {
    docsPage: true,
  },
};
