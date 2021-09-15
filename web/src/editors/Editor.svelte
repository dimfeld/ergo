<script context="module" lang="ts">
  // This is the same as CodeMirror's LintSource type but it's not currently exported.
  export type LintSource = (
    view: EditorView
  ) => readonly Diagnostic[] | Promise<readonly Diagnostic[]>;
</script>

<script lang="ts">
  import { createEventDispatcher, setContext } from 'svelte';
  import {
    EditorView,
    keymap,
    highlightSpecialChars,
    drawSelection,
    highlightActiveLine,
    ViewUpdate,
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
  import { Diagnostic, linter as makeLinter, lintKeymap } from '@codemirror/lint';
  import { javascript } from '@codemirror/lang-javascript';
  import { json, jsonParseLinter } from '@codemirror/lang-json';
  import { json5, json5Linter } from './codemirror-json5';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { darkModeStore, cssDarkModePreference } from '^/styles';
  import throttle from 'just-throttle';

  export let contents: string;
  export let format: 'js' | 'json' | 'json5';
  export let enableWrapping = true;
  export let notifyOnChange = false;

  export let linter: LintSource | undefined = undefined;

  const dispatch = createEventDispatcher<{ change: string }>();

  let language = new Compartment();
  let lineWrapping = new Compartment();
  let theme = new Compartment();

  const darkMode = darkModeStore();

  const languages = {
    js: javascript,
    json,
    json5,
  };

  const linters = {
    json: jsonParseLinter,
    json5: json5Linter,
  };

  export const view = new EditorView({
    state: EditorState.create({
      doc: contents,
      extensions: [
        EditorView.updateListener.of(viewUpdated),
        language.of([]),
        lineWrapping.of([]),
        theme.of([]),
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

  setContext('editorView', view);

  const notifyDocChanged = throttle(() => dispatch('change', view.state.doc.toString()), 500, {
    trailing: true,
  });

  function viewUpdated(update: ViewUpdate) {
    if (notifyOnChange && update.docChanged) {
      notifyDocChanged();
    }
  }

  function updateCompartment(compartment: Compartment, value: Extension) {
    view.dispatch({
      effects: compartment.reconfigure(value),
    });
  }

  $: activeLinter = linter ?? linters[format]();
  $: languageComponents = [
    languages[format](),
    activeLinter ? makeLinter(activeLinter) : undefined,
  ].filter(Boolean);

  $: updateCompartment(language, languageComponents);
  $: updateCompartment(lineWrapping, enableWrapping ? [EditorView.lineWrapping] : []);
  $: updateCompartment(theme, $darkMode ?? cssDarkModePreference() ? [oneDark] : []);

  function editor(node: HTMLDivElement) {
    node.appendChild(view.dom);
    return {
      destroy: () => view.destroy(),
    };
  }
</script>

<div class="editor h-full flex flex-col">
  <div class="h-6 py-1 flex w-full text-sm border-b border-gray-200 dark:border-gray-800">
    <div class="ml-auto">
      <label><input type="checkbox" bind:checked={enableWrapping} /> Wrap</label>
    </div>
  </div>
  <div class="flex-grow" use:editor />
  <slot />
</div>

<style>
  .editor :global(.cm-scroller) {
    overflow: auto;
  }

  .editor :global(.cm-editor) {
    height: 100%;
  }
</style>
