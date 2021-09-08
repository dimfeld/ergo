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
  import type { Extension } from '@codemirror/state';
  import { Compartment, EditorState } from '@codemirror/state';
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
  import { oneDark } from '@codemirror/theme-one-dark';
  import { darkModeStore, cssDarkModePreference } from '^/styles';
  import throttle from 'just-throttle';

  export let contents: string;
  export let format: 'js' | 'json';
  export let enableWrapping = true;
  export let notifyOnChange = false;

  const dispatch = createEventDispatcher<{ change: string }>();

  let language = new Compartment();
  let lineWrapping = new Compartment();
  let theme = new Compartment();

  const darkMode = darkModeStore();

  const languages = {
    js: javascript,
    json: json,
  };

  export const view = new EditorView({
    state: EditorState.create({
      doc: contents,
      extensions: [
        EditorView.updateListener.of(viewUpdated),
        language.of(languages[format]()),
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

  $: updateCompartment(language, languages[format]());
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
