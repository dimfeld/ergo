const tailwindcss = require('tailwindcss');
const nesting = require('tailwindcss/nesting');
const autoprefixer = require('autoprefixer');
const nested = require('postcss-nested');
const cssnano = require('cssnano');
const tailwindConfig = require('./tailwind.config.cjs');

const mode = process.env.NODE_ENV;
const dev = mode === 'development';

const config = {
  plugins: [
    nesting(),
    tailwindcss(tailwindConfig),
    autoprefixer(),
    !dev &&
      cssnano({
        preset: 'default',
      }),
  ].filter(Boolean),
};

module.exports = config;
