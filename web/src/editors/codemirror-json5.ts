import { parser as lezerParser } from 'lezer-json5';
import { parse as fullParser } from 'json5';
import {
  continuedIndent,
  indentNodeProp,
  foldNodeProp,
  foldInside,
  LRLanguage,
  LanguageSupport,
} from '@codemirror/language';
import { styleTags, tags as t } from '@codemirror/highlight';
import { EditorView } from '@codemirror/view';
import { Diagnostic } from '@codemirror/lint';

/// A language provider that provides JSON5 parsing.
export const jsonLanguage = LRLanguage.define({
  parser: lezerParser.configure({
    props: [
      indentNodeProp.add({
        Object: continuedIndent({ except: /^\s*\}/ }),
        Array: continuedIndent({ except: /^\s*\]/ }),
      }),
      foldNodeProp.add({
        'Object Array': foldInside,
      }),
      styleTags({
        String: t.string,
        Number: t.number,
        'True False': t.bool,
        Identifier: t.propertyName,
        null: t.null,
        ',': t.separator,
        '[ ]': t.squareBracket,
        '{ }': t.brace,
      }),
    ],
  }),
  languageData: {
    closeBrackets: { brackets: ['[', '{', '"', `'`] },
    indentOnInput: /^\s*[\}\]]$/,
  },
});

/// JSON5 language support.
export function json5() {
  return new LanguageSupport(jsonLanguage);
}

export function json5Linter() {
  return (view: EditorView): Diagnostic[] => {
    let doc = view.state.doc;
    try {
      fullParser(doc.toString());
      return [];
    } catch (e: any) {
      let pos = 0;
      if ('lineNumber' in e && 'columnNumber' in e) {
        pos = Math.min(doc.line(e.lineNumber).from + e.columnNumber - 1, doc.length);
      }

      return [
        {
          from: pos,
          to: pos,
          message: e.message,
          severity: 'error',
        },
      ];
    }
  };
}
