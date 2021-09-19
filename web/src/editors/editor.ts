import { EditorState } from '@codemirror/state';
import { CompletionContext } from '@codemirror/autocomplete';
import { syntaxTree } from '@codemirror/language';
import { SyntaxNode, Tree } from '@lezer/common';

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

function propertyKey(state: EditorState, node: SyntaxNode) {
  console.log('propertyKey', node.name);
  if (node.name !== 'Property') {
    return null;
  }

  let key = node.getChild('PropertyName');
  if (!key) {
    return null;
  }

  return stripQuotes(state.sliceDoc(key.from, key.to));
}

function propertyValueNode(node: SyntaxNode) {
  if (!node || node.name !== 'Property') {
    return null;
  }

  return validNodeLookLeft(node.lastChild);
}

function getParentProperty(node: SyntaxNode) {
  let cursor = node.parent?.cursor;
  if (!cursor) {
    return null;
  }

  while (cursor.node.name !== 'Property' && cursor.node.name !== 'Array') {
    if (!cursor.parent()) {
      return null;
    }
  }

  return cursor.node;
}

function getPathAtNode(state: EditorState, node: SyntaxNode) {
  let keys: (string | number)[] = [];

  while (true) {
    let parent = getParentProperty(node);
    if (!parent) {
      break;
    }

    console.log('Parent', parent.name, state.sliceDoc(parent.from, parent.to));

    if (parent.name === 'Array') {
      let thisPos = node.from;
      let findNode = parent.firstChild;
      let index = 0;
      while (findNode && findNode.to < thisPos) {
        findNode = validNodeLookRight(findNode.nextSibling);
        index += 1;
      }

      keys.unshift(index);
    } else {
      let key = propertyKey(state, parent);
      if (key) {
        keys.unshift(key);
      }
    }
    node = parent;
  }

  return keys;
}

/** If the passed node is a comment, go left until a non-comment node is found */
function validNodeLookLeft(node: SyntaxNode | null) {
  while (node && /Comment/.test(node.name)) {
    node = node.prevSibling;
  }
  return node;
}

/** If the passed node is a comment, go right until a non-comment node is found */
function validNodeLookRight(node: SyntaxNode | null) {
  while (node && /Comment/.test(node.name)) {
    node = node.nextSibling;
  }
  return node;
}

function findObjectProperty(state: EditorState, node: SyntaxNode | null, name: string) {
  console.log('findObjectProperty', node?.name);
  if (node?.name === 'Property') {
    node = propertyValueNode(node);
  }

  if (node?.name !== 'Object') {
    return null;
  }

  let cursor = node.firstChild?.cursor;
  if (!cursor) {
    return null;
  }

  while (propertyKey(state, cursor.node) !== name) {
    if (!cursor.nextSibling()) {
      return null;
    }
  }

  return cursor.node;
}

function findArrayIndex(node: SyntaxNode | null, index: number) {
  if (node?.name === 'Property') {
    node = propertyValueNode(node);
  }

  console.log('findArrayIndex', node?.name, index);
  if (node?.name !== 'Array') {
    return null;
  }

  let cursor = node.firstChild?.cursor;
  if (!cursor) {
    return null;
  }

  for (let i = 1; i < index; ++i) {
    if (!cursor.nextSibling()) {
      return null;
    }
  }

  return cursor.node;
}

// TODO Cache partial tree lookups for multiple calls on a tree.
function nodeFromPath(state: EditorState, tree: Tree, path: (string | number)[]) {
  let node: SyntaxNode | null = tree.topNode.firstChild;

  let i: number;
  for (i = 0; i < path.length; ++i) {
    let desired = path[i];

    if (typeof desired === 'number') {
      node = findArrayIndex(node, desired);
    } else {
      node = findObjectProperty(state, node, desired);
    }

    if (!node) {
      break;
    }
  }

  let key = node?.getChild('PropertyName');
  let value = propertyValueNode(node);

  let found = i === path.length;
  return {
    found,
    path: found ? path : path.slice(0, i),
    property: node,
    key: key
      ? {
          from: key.from,
          to: key.to,
          text: state.sliceDoc(key.from, key.to),
        }
      : null,
    value: value
      ? {
          from: value.from,
          to: value.to,
          text: state.sliceDoc(value.from, value.to),
        }
      : null,
  };
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

    let isKey = pos.name === 'PropertyName';
    console.dir({
      pos,
      tree,
      path,
      name: nodeName,
      isKey,
      lookup: nodeFromPath(context.state, tree, path),
    });
    return null;
  };
}
