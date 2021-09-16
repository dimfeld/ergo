import { parser as lezerParser } from 'lezer-json5';
import JSON5 from 'json5';
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
export const json5Language = LRLanguage.define({
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
        'PropertyName!': t.propertyName,
        Number: t.number,
        'True False': t.bool,
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
  return new LanguageSupport(json5Language);
}

/** A function to provide additional linting functionality on the parsed version of the object */
export type StructureLinter<T = unknown> = (
  view: EditorView,
  parsed: T
) => Diagnostic[] | Promise<Diagnostic[]>;

/**
 * JSON5 linting support
 *
 * @param structureLinter Perform additional linting on the parsed object
 **/
export function json5ParseLinter<T = unknown>(structureLinter?: StructureLinter<T>) {
  return (view: EditorView): Diagnostic[] | Promise<Diagnostic[]> => {
    let doc = view.state.doc;
    try {
      let parsed = JSON5.parse(doc.toString());
      return structureLinter?.(view, parsed) ?? [];
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
