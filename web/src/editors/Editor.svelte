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
  import { Compartment, EditorSelection, EditorState, Extension } from '@codemirror/state';
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
  import { json5, json5ParseLinter } from './codemirror-json5';
  import { oneDark } from '@codemirror/theme-one-dark';
  import prettier from 'prettier/standalone';
  import prettierBabel from 'prettier/parser-babel';

  import { darkModeStore, cssDarkModePreference } from '^/styles';
  import throttle from 'just-throttle';

  import Button from '^/components/Button.svelte';

  import { autocompleter, AutocompleteSpec } from './autocomplete';
  import { LintSource } from './editor';
  import { PanelConstructor, showPanel } from '@codemirror/panel';
  import { jsonSchemaSupport } from './json_schema';
  import type { JSONSchema4, JSONSchema6, JSONSchema7 } from 'json-schema';

  export let contents: string;
  export let format: 'js' | 'json' | 'json5';
  export let enableWrapping = true;
  export let notifyOnChange = false;
  export let jsonSchema: JSONSchema4 | JSONSchema6 | JSONSchema7 | undefined = undefined;

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
    json5: json5ParseLinter,
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

  function runPrettier() {
    let currentCursor = view.state.selection.ranges[view.state.selection.mainIndex].to;
    let newText = prettier.formatWithCursor(view.state.doc.toString(), {
      cursorOffset: currentCursor,
      parser: format,
      plugins: [prettierBabel],
    });

    view.dispatch({
      changes: [{ from: 0, to: view.state.doc.length, insert: newText.formatted }],
      selection: EditorSelection.cursor(newText.cursorOffset),
    });
    view.focus();
  }

  $: activeLinter = linter ?? linters[format]?.();
  $: lintExtension = activeLinter ? makeLinter(activeLinter) : undefined;

  $: jsonSchemaComponents = jsonSchema ? jsonSchemaSupport(jsonSchema) : null;

  $: autocompleteExtension = autocompletion({
    activateOnTyping: true,
    override: [
      autocompleter([jsonSchemaComponents?.autocomplete].filter(Boolean) as AutocompleteSpec[]),
    ],
  });
  $: jsonSchemaPanel = jsonSchemaComponents?.panel
    ? showPanel.of(jsonSchemaComponents.panel)
    : null;
  $: languageComponents = [
    languages[format](),
    lintExtension,
    autocompleteExtension,
    jsonSchemaPanel,
  ].filter(Boolean) as Extension[];

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
  <div class="py-1 flex w-full text-sm border-b border-gray-200 dark:border-gray-800">
    <div class="ml-auto flex flex-row space-x-4 items-center">
      <Button size="xs" on:click={runPrettier}>Format</Button>
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
