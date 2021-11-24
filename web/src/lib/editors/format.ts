import prettier from 'prettier/standalone';
import prettierBabel from 'prettier/parser-babel';

export function formatJson(s: object | string, parser: 'json' | 'json5' = 'json5') {
  return prettier.format(typeof s === 'string' ? s : JSON.stringify(s), {
    parser,
    plugins: [prettierBabel],
  });
}
