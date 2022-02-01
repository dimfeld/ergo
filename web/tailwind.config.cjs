const defaultTheme = require('tailwindcss/defaultTheme');
const colors = require('tailwindcss/colors');
const formsPlugin = require('@tailwindcss/forms');
const { autoDarkPlugin, autoDarkColors } = require('./tailwind-autodark.cjs');

const config = {
  mode: 'jit',
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        accent: colors.orange,
        ...autoDarkColors({ colors: ['gray', 'accent'] }),
      },
      fontFamily: {
        sans: ['Inter var', ...defaultTheme.fontFamily.sans],
      },
    },
  },
  plugins: [
    formsPlugin,
    autoDarkPlugin({
      mainElement: 'body',
      colors: ['gray', 'accent'],
    }),
  ],
  darkMode: 'class',
};

module.exports = config;
