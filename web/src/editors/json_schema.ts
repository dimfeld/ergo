import { AutocompleteSpec } from './autocomplete';
import { PanelConstructor } from '@codemirror/panel';
import { jsonCursorPath } from './codemirror-json5';
import type { JSONSchema4, JSONSchema6, JSONSchema7 } from 'json-schema';

function formatPath(path: (string | number)[] | null) {
  if (!path) {
    return '';
  }

  return path
    .map((p, i) => {
      if (i === 0) {
        return p;
      } else if (typeof p === 'string') {
        return '.' + p;
      } else {
        return `[${p}]`;
      }
    })
    .join('');
}

export function jsonSchemaSupport(jsonSchema: JSONSchema4 | JSONSchema6 | JSONSchema7): {
  autocomplete: AutocompleteSpec;
  panel: PanelConstructor;
} {
  return {
    autocomplete: {
      values: (obj, currentPath) => {
        return [];
      },
    },
    panel: (_view) => {
      let dom = document.createElement('div');
      return {
        dom,
        update(update) {
          let { path } = update.state.field(jsonCursorPath);
          dom.textContent = formatPath(path);
        },
      };
    },
  };
}
