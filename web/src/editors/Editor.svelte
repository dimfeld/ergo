<script lang="ts">
  import {
    EditorView,
    keymap,
    highlightSpecialChars,
    drawSelection,
    highlightActiveLine,
  } from '@codemirror/view';
  import { Compartment, EditorState, Extension } from '@codemirror/state';
  import { history, historyKeymap } from '@codemirror/history';
  import { indentOnInput } from '@codemirror/language';
  import { lineNumbers, highlightActiveLineGutter } from '@codemirror/gutter';
  import { defaultKeymap, indentWithTab } from '@codemirror/commands';
  import { bracketMatching } from '@codemirror/matchbrackets';
  import { closeBrackets, closeBracketsKeymap } from '@codemirror/closebrackets';
  import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
  import { autocompletion, completionKeymap } from '@codemirror/autocomplete';
  import { commentKeymap } from '@codemirror/comment';
  import { defaultHighlightStyle } from '@codemirror/highlight';
  import { lintKeymap } from '@codemirror/lint';
  import { javascript } from '@codemirror/lang-javascript';
  import { json } from '@codemirror/lang-json';

  export let contents: string;
  export let format: 'js' | 'json';

  let language = new Compartment();

  const languages = {
    js: javascript,
    json: json,
  };

  export const view = new EditorView({
    state: EditorState.create({
      doc: contents,
      extensions: [
        language.of(languages[format]()),
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightSpecialChars(),
        history(),
        drawSelection(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        defaultHighlightStyle.fallback,
        bracketMatching(),
        closeBrackets(),
        autocompletion(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        EditorView.lineWrapping,
        keymap.of([
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          ...commentKeymap,
          ...completionKeymap,
          ...lintKeymap,
          indentWithTab,
        ]),
      ],
    }),
  });

  function updateEditorState(compartment: Compartment, value: Extension) {
    view.dispatch({
      effects: compartment.reconfigure(value),
    });
  }

  $: updateEditorState(language, languages[format]());

  function editor(node: HTMLDivElement) {
    node.appendChild(view.dom);
    return {
      destroy: () => view.destroy(),
    };
  }
</script>

<div use:editor />
