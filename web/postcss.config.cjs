const tailwindcss = require('tailwindcss');
const autoprefixer = require('autoprefixer');
const nested = require('postcss-nested');
const cssnano = require('cssnano');
const tailwindConfig = require('./tailwind.config.cjs');

const mode = process.env.NODE_ENV;
const dev = mode === 'development';

const config = {
  plugins: [
    //Some plugins, like postcss-nested, need to run before Tailwind,
    nested(),
    tailwindcss(tailwindConfig),
    //But others, like autoprefixer, need to run after,
    autoprefixer(),
    !dev &&
      cssnano({
        preset: 'default',
      }),
  ].filter(Boolean),
};

module.exports = config;
