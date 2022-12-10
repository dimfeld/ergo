import { type LintSource, nodeFromPath } from './editor';
import { json5ParseLinter } from './codemirror-json5';
import { syntaxTree } from '@codemirror/language';

export interface ObjectLintResult {
  path: string[];
  /** true to place the diagnostic on the key. If false or omitted, the diagnostic
   * is placed on the value. */
  key: boolean;
  message: string;
  severity?: 'info' | 'warning' | 'error';
}

export type ObjectLinter<T> = (obj: T) => ObjectLintResult[] | undefined;

export function objectLinter<T>(lintFunc: ObjectLinter<T>): LintSource {
  return json5ParseLinter<T>((view, obj) => {
    let result = lintFunc(obj);
    if (!result) {
      return [];
    }

    return result.map((r) => {
      let state = view.state;
      let node = nodeFromPath(state, syntaxTree(state), r.path);
      let from: number;
      let to: number;

      if (r.key && node.key) {
        ({ from, to } = node.key);
      } else if (!r.key && node.value) {
        ({ from, to } = node.value);
      } else if (node.property) {
        ({ from, to } = node.property);
      } else {
        from = 0;
        to = state.doc.length;
      }

      return {
        from,
        to,
        message: r.message,
        severity: r.severity ?? 'error',
      };
    });
  });
}
