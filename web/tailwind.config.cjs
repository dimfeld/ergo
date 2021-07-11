const defaultTheme = require('tailwindcss/defaultTheme');
const colors = require('tailwindcss/colors');
const formsPlugin = require('@tailwindcss/forms');

const config = {
  mode: 'jit',
  purge: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        accent: colors.orange,
      },
      fontFamily: {
        sans: ['Inter var', ...defaultTheme.fontFamily.sans],
      },
    },
  },
  plugins: [formsPlugin],
  darkMode: 'class',
};

module.exports = config;
