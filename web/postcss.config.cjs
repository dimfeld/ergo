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
    // When Tailwind transforms `.cl { @apply dark:text-gray-200 }` into
    // `.dark .cl { ... }, replace .dark with :global(.dark) in Svelte files.
    function(css) {
      if(css.source.input.file.endsWith('.svelte')) {
        css.walkRules((rule) => {
          rule.selectors = rule.selectors.map((selector) => {
            return selector.replace(/^\.dark /, ':global(.dark) ');
          })
        });
      }
    },
    //But others, like autoprefixer, need to run after,
    autoprefixer(),
    !dev &&
      cssnano({
        preset: 'default',
      }),
  ].filter(Boolean),
};

module.exports = config;
