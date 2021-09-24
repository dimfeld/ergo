import { CompletionContext } from '@codemirror/autocomplete';
import { syntaxTree } from '@codemirror/language';
import { jsonCursorPath } from './codemirror-json5';
import { nodeFromPath } from './editor';

export interface AutocompleteSpec<T = unknown> {
  path?: string[] | string | RegExp;
  values: (obj: T, currentPath: string[]) => string[];
}

export function autocompleter<T>(specs: AutocompleteSpec<T>[]) {
  return (context: CompletionContext) => {
    let { path, node } = context.state.field(jsonCursorPath);
    if (!node) {
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
    console.dir({
      node,
      path,
      name: nodeName,
      inKey,
      lookup: path ? nodeFromPath(context.state, tree, path) : null,
    });
    return null;
  };
}
