import { EditorState } from '@codemirror/state';
import { CompletionContext } from '@codemirror/autocomplete';
import { syntaxTree } from '@codemirror/language';
import { SyntaxNode } from '@lezer/common';

export interface AutocompleteSpec<T = unknown> {
  tokensBefore?: string[];
  values: (obj: T, currentPath: string[]) => string[];
}

function stripQuotes(s: string) {
  let first = s[0];
  let last = s[s.length - 1];
  if ((first === `'` && last === `'`) || (first === `"` && last === `"`)) {
    s = s.slice(1, -1);
  }
  return s;
}

function prevToken(node: SyntaxNode) {
  let prev = node.cursor.moveTo(node.from, -1);
  while (/Comment/.test(prev.name)) {
    prev = prev.cursor.moveTo(prev.from, -1);
  }
  return prev?.node;
}

function memberKey(state: EditorState, node: SyntaxNode) {
  if (node.name !== 'Member') {
    return null;
  }
  let key = node.getChild('PropertyName');
  if (!key) {
    return null;
  }

  return stripQuotes(state.sliceDoc(key.from, key.to));
}

function getParentMember(node: SyntaxNode) {
  let cursor = node.cursor;
  if (!cursor.parent()) {
    return null;
  }

  while (cursor.node.name !== 'Member') {
    if (!cursor.parent()) {
      return null;
    }
  }

  return cursor.node;
}

function getTreePath(state: EditorState, node: SyntaxNode) {
  let keys = [];

  while (true) {
    node = getParentMember(node);
    if (!node) {
      break;
    }

    let key = memberKey(state, node);
    if (key) {
      keys.unshift(key);
    }
  }

  return keys;
}

export function autocompleter<T>(specs: AutocompleteSpec<T>[]) {
  return (context: CompletionContext) => {
    let tree = syntaxTree(context.state);
    let pos = tree.resolveInner(context.pos, -1);
    let path = getTreePath(context.state, pos);

    let nodeName = pos.name;
    if (nodeName === 'Member') {
      // We're somewhere in whitespace inside the Member, so figure out what the
      // previous node was.
      let childBeforeCursor = pos.childBefore(context.pos);
      if (childBeforeCursor) {
        nodeName = childBeforeCursor.name;
      }
    }

    let isKey = pos.name === 'PropertyName';
    console.dir({ pos, tree, path, name: nodeName, isKey });
    return null;
  };
}
