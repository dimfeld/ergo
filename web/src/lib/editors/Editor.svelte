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

  import { darkModeStore, cssDarkModePreference } from '$lib/styles';
  import throttle from 'just-throttle';

  import Button from '$lib/components/Button.svelte';

  import { autocompleter, AutocompleteSpec } from './autocomplete';
  import { injectTsTypes, LintSource } from './editor';
  import { PanelConstructor, showPanel } from '@codemirror/panel';
  import { jsonSchemaSupport } from './json_schema';
  import type { JSONSchema4, JSONSchema6, JSONSchema7 } from 'json-schema';
  import { FileMap, typescript } from './typescript';

  export let contents: string;
  export let format: 'js' | 'ts' | 'json' | 'json5';
  export let enableWrapping = true;
  export let notifyOnChange = false;
  export let jsonSchema: JSONSchema4 | JSONSchema6 | JSONSchema7 | undefined = undefined;
  export let tsDefs: FileMap | undefined = undefined;

  export let linter: LintSource | undefined = undefined;

  const dispatch = createEventDispatcher<{ change: string }>();

  let language = new Compartment();
  let lintCompartment = new Compartment();
  let lineWrapping = new Compartment();
  let theme = new Compartment();

  const darkMode = darkModeStore();

  interface LanguageSupport {
    extension: () => Extension;
    linter?: () => LintSource;
    autocomplete?: () => Extension;
    prettierParser: string;
  }

  $: jsonSchemaComponents = jsonSchema ? jsonSchemaSupport(jsonSchema) : null;

  function jsonSchemaAutocomplete() {
    return autocompletion({
      activateOnTyping: true,
      override: [
        autocompleter([jsonSchemaComponents?.autocomplete].filter(Boolean) as AutocompleteSpec[]),
      ],
    });
  }

  const languages: Record<string, LanguageSupport> = {
    js: {
      extension: typescript,
      prettierParser: 'babel',
    },
    ts: {
      extension: typescript,
      prettierParser: 'babel-ts',
    },
    json: {
      extension: json,
      linter: jsonParseLinter,
      autocomplete: jsonSchemaAutocomplete,
      prettierParser: 'json',
    },
    json5: {
      extension: json5,
      linter: json5ParseLinter,
      autocomplete: jsonSchemaAutocomplete,
      prettierParser: 'json5',
    },
  };

  export function getContents() {
    return view.state.doc.toString();
  }

  export const view = new EditorView({
    state: EditorState.create({
      doc: contents,
      extensions: [
        EditorView.updateListener.of(viewUpdated),
        language.of([]),
        lintCompartment.of([]),
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
          // indentWithTab,
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
      parser: languages[format].prettierParser,
      plugins: [prettierBabel],
    });

    view.dispatch({
      changes: [{ from: 0, to: view.state.doc.length, insert: newText.formatted }],
      selection: EditorSelection.cursor(newText.cursorOffset),
    });
    view.focus();
  }

  $: activeLinter = linter ?? languages[format]?.linter?.();
  $: lintExtension = activeLinter ? makeLinter(activeLinter) : undefined;

  $: jsonSchemaPanel = jsonSchemaComponents?.panel
    ? showPanel.of(jsonSchemaComponents.panel)
    : null;
  $: languageComponents = [
    languages[format].extension(),
    lintExtension,
    languages[format].autocomplete?.(),
    jsonSchemaPanel,
  ].filter(Boolean) as Extension[];

  $: updateCompartment(language, languageComponents);

  $: updateCompartment(lintCompartment, [lintExtension].filter(Boolean) as Extension[]);
  $: updateCompartment(lineWrapping, enableWrapping ? [EditorView.lineWrapping] : []);
  $: updateCompartment(theme, $darkMode ?? cssDarkModePreference() ? [oneDark] : []);

  $: injectTsTypes(view, tsDefs ?? {});

  function editor(node: HTMLDivElement) {
    node.appendChild(view.dom);
    return {
      destroy: () => view.destroy(),
    };
  }
</script>

<div class="editor h-full min-h-0 flex flex-col">
  <div class="py-1 flex w-full text-sm border-b border-gray-200 dark:border-gray-800">
    <div class="ml-auto flex flex-row space-x-4 items-center">
      <Button size="xs" on:click={runPrettier}>Format</Button>
      <label><input type="checkbox" bind:checked={enableWrapping} /> Wrap</label>
    </div>
  </div>
  <div class="min-h-0 flex-1 flex flex-col" use:editor />
  <slot />
</div>

<style>
  .editor :global(.cm-scroller) {
    overflow: auto;
  }

  .editor :global(.cm-editor) {
    min-height: 0;
    height: 100%;
  }
</style>
