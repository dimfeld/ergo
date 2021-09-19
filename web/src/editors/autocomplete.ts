import { CompletionContext } from '@codemirror/autocomplete';
import { syntaxTree } from '@codemirror/language';
import { getPathAtNode, nodeFromPath } from './editor';

export interface AutocompleteSpec<T = unknown> {
  path: string[] | string | RegExp;
  values: (obj: T, currentPath: string[]) => string[];
}

export function autocompleter<T>(specs: AutocompleteSpec<T>[]) {
  return (context: CompletionContext) => {
    let tree = syntaxTree(context.state);
    let pos = tree.resolveInner(context.pos, -1);
    let path = getPathAtNode(context.state, pos);

    let nodeName = pos.name;
    if (nodeName === 'Property') {
      // We're somewhere in whitespace inside the Property, so figure out what the
      // previous node was.
      let childBeforeCursor = pos.childBefore(context.pos);
      if (childBeforeCursor) {
        nodeName = childBeforeCursor.name;
      }
    }

    let inKey = pos.name === 'PropertyName';
    console.dir({
      pos,
      tree,
      path,
      name: nodeName,
      inKey,
      lookup: nodeFromPath(context.state, tree, path),
    });
    return null;
  };
}
