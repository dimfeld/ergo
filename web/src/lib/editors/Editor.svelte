<script lang="ts">
  import {
    autocompletion,
    closeBrackets,
    closeBracketsKeymap,
    completionKeymap,
  } from '@codemirror/autocomplete';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import { json, jsonParseLinter } from '@codemirror/lang-json';
  import {
    bracketMatching,
    defaultHighlightStyle,
    indentOnInput,
    syntaxHighlighting,
  } from '@codemirror/language';
  import { linter as makeLinter, lintKeymap } from '@codemirror/lint';
  import { highlightSelectionMatches, searchKeymap } from '@codemirror/search';
  import { Compartment, EditorSelection, EditorState, type Extension } from '@codemirror/state';
  import { oneDark } from '@codemirror/theme-one-dark';
  import {
    drawSelection,
    EditorView,
    highlightActiveLine,
    highlightActiveLineGutter,
    highlightSpecialChars,
    keymap,
    lineNumbers,
    showPanel,
    type ViewUpdate,
  } from '@codemirror/view';
  import prettierBabel from 'prettier/parser-babel';
  import prettier from 'prettier/standalone';
  import { autocompleter, type AutocompleteSpec } from './autocomplete';
  import { json5, json5ParseLinter } from 'codemirror-json5';
  import { injectTsTypes, type LintSource } from './editor';

  import { cssDarkModePreference, darkModeStore } from '$lib/styles';
  import { throttle } from 'lodash-es';
  import { createEventDispatcher, setContext } from 'svelte';

  import Button from '$lib/components/Button.svelte';

  import * as bundler from '$lib/bundler/index';
  import Card from '$lib/components/Card.svelte';
  import type { JSONSchema4, JSONSchema6, JSONSchema7 } from 'json-schema';
  import { jsonSchemaSupport } from './json_schema';
  import { typescript, type FileMap, type WrapCodeFn } from './typescript';

  export let contents: string;
  export let format: 'js' | 'ts' | 'json' | 'json5';
  export let enableWrapping = true;
  export let notifyOnChange = false;
  export let jsonSchema: JSONSchema4 | JSONSchema6 | JSONSchema7 | undefined = undefined;
  export let tsDefs: FileMap | undefined = undefined;
  export let wrapCode: WrapCodeFn | undefined = undefined;
  export let toolbar = true;

  let classNames = '';
  export { classNames as class };

  export let linter: LintSource | undefined = undefined;

  const dispatch = createEventDispatcher<{ change: string }>();

  let languageCompartment = new Compartment();
  let lintCompartment = new Compartment();
  let lineWrapping = new Compartment();
  let theme = new Compartment();

  const darkMode = darkModeStore();

  interface LanguageSupport {
    extension: () => Extension;
    linter?: () => LintSource;
    autocomplete?: () => Extension;
    prettierParser: string;
    compilable?: boolean;
    injectTsTypes?: boolean;
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
      extension: () => typescript(wrapCode),
      prettierParser: 'babel',
      compilable: true,
      injectTsTypes: true,
    },
    ts: {
      extension: () => typescript(wrapCode),
      prettierParser: 'babel-ts',
      compilable: true,
      injectTsTypes: true,
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

  $: language = languages[format];

  export function getContents() {
    return view.state.doc.toString();
  }

  export const view = new EditorView({
    state: EditorState.create({
      doc: contents,
      extensions: [
        EditorView.updateListener.of(viewUpdated),
        languageCompartment.of([]),
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
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        bracketMatching(),
        closeBrackets(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        keymap.of([
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          ...completionKeymap,
          ...lintKeymap,
          // indentWithTab,
        ]),
      ],
    }),
  });

  setContext('editorView', view);

  const notifyDocChanged = throttle(() => dispatch('change', view.state.sliceDoc(0)), 100);

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
      parser: language.prettierParser,
      plugins: [prettierBabel],
    });

    view.dispatch({
      changes: [{ from: 0, to: view.state.doc.length, insert: newText.formatted }],
      selection: EditorSelection.cursor(newText.cursorOffset),
    });
    view.focus();
  }

  let bundlePreview: bundler.Result | null = null;
  async function previewCompile() {
    let code = view.state.sliceDoc(0);
    let worker = new bundler.Bundler();

    console.log(`Compiling`, code);
    bundlePreview = null;
    try {
      let result = await worker.bundle({
        files: {
          'index.ts': code,
        },
      });

      bundlePreview = result;
    } finally {
      worker.destroy();
    }
  }

  $: activeLinter = linter ?? language?.linter?.();
  $: lintExtension = activeLinter ? makeLinter(activeLinter) : undefined;

  $: jsonSchemaPanel = jsonSchemaComponents?.panel
    ? showPanel.of(jsonSchemaComponents.panel)
    : null;
  $: languageComponents = [
    language.extension(),
    lintExtension,
    language.autocomplete?.(),
    jsonSchemaPanel,
  ].filter(Boolean) as Extension[];

  $: updateCompartment(languageCompartment, languageComponents);

  $: updateCompartment(lintCompartment, [lintExtension].filter(Boolean) as Extension[]);
  $: updateCompartment(lineWrapping, enableWrapping ? [EditorView.lineWrapping] : []);
  $: updateCompartment(theme, $darkMode ?? cssDarkModePreference() ? [oneDark] : []);

  $: if (language?.injectTsTypes || tsDefs) {
    injectTsTypes(view, tsDefs ?? {});
  }

  function editor(node: HTMLDivElement) {
    node.appendChild(view.dom);
    return {
      destroy: () => view.destroy(),
    };
  }
</script>

<div class="editor flex h-full min-h-0 flex-col {classNames}">
  {#if toolbar}
    <div
      class="flex w-full items-center border-b border-gray-200 py-1 text-sm dark:border-gray-800">
      <slot name="left-toolbar" />
      <div class="ml-auto flex flex-row items-center space-x-4">
        <slot name="right-toolbar" />
        {#if language.compilable}
          <!-- for use while this feature is in early development -->
          <Button size="xs" on:click={previewCompile}>Preview Compile</Button>
        {/if}
        <Button size="xs" on:click={runPrettier}>Format</Button>
        <label><input type="checkbox" bind:checked={enableWrapping} /> Wrap</label>
      </div>
    </div>
  {/if}
  <div class="flex min-h-0 flex-1 flex-col" use:editor />
  <slot />
</div>

{#if bundlePreview}
  <Card class="mt-8" label="Bundle Preview {bundlePreview.error ? '(Error)' : ''}">
    {#if bundlePreview.error}
      {bundlePreview.error.message}
    {:else}
      {#if bundlePreview.warnings?.length}
        <h3>Warnings</h3>
        <ul>
          {#each bundlePreview.warnings as warning}
            <li>{warning}</li>
          {/each}
        </ul>
      {/if}

      <textarea class="h-48 w-full">
        {bundlePreview.code}
      </textarea>
    {/if}
  </Card>
{/if}

<style>
  .editor :global(.cm-scroller) {
    overflow: auto;
  }

  .editor :global(.cm-editor) {
    min-height: 0;
    height: 100%;
  }
</style>
