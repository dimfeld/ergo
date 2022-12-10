import type { Completion, CompletionContext } from '@codemirror/autocomplete';
import { syntaxTree } from '@codemirror/language';
import { json5ParseCache, jsonCursorPath } from 'codemirror-json5';
import { nodeFromPath } from './editor';

export interface AutocompleteSpec<T = unknown> {
  path?: string[];
  matchPathPrefix?: boolean;
  values: (
    obj: T,
    currentPath: (string | number)[],
    inKey: boolean
  ) => (Completion | string)[] | null | undefined;
}

export function autocompleter<T>(specs: AutocompleteSpec<T>[]) {
  return (context: CompletionContext) => {
    let obj = context.state.field(json5ParseCache, false)?.obj;
    if (!obj) {
      return null;
    }

    const { path, node } = context.state.field(jsonCursorPath);
    if (!path || !node) {
      return null;
    }

    let nodeName = node.name;
    if (nodeName === 'Property') {
      // We're somewhere in whitespace inside the Property, so figure out what the
      // previous node was.
      let childBeforeCursor = node.childBefore(context.pos);
      if (childBeforeCursor) {
        nodeName = childBeforeCursor.name;
      }
    }

    let inKey = node.name === 'PropertyName';
    let tree = syntaxTree(context.state);

    let completions = specs
      .filter((spec) => {
        if (!spec.path) {
          return true;
        }

        if (!spec.matchPathPrefix && path.length !== spec.path.length) {
          return false;
        }

        for (let i = 0; i < spec.path.length; i++) {
          let specPath = spec.path[i];
          if (path[i] !== specPath && specPath !== '*') {
            return false;
          }
        }

        return true;
      })
      .flatMap((spec) => spec.values(obj as T, path, inKey) ?? []);

    console.dir({
      node,
      path,
      name: nodeName,
      inKey,
      lookup: path ? nodeFromPath(context.state, tree, path) : null,
      completions,
    });

    if (!completions.length) {
      return null;
    }

    return {
      from: node.from,
      options: completions.map((c) => {
        if (typeof c === 'string') {
          return {
            label: c,
          };
        }

        return c;
      }),
    };
  };
}
