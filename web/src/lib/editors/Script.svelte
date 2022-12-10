<script lang="ts">
  import type { EditorView } from '@codemirror/view';
  import type { TaskAction, TaskTrigger } from '../api_types';
  import { baseData } from '../data';
  import { scriptTypeDefinitions } from './types/task_script_definitions';
  import { logger } from '../logger';
  import Editor from './Editor.svelte';
  import ScriptSimulator from '$lib/script_simulator/ScriptSimulator.svelte';
  import { Bundler } from '$lib/bundler';
  import { onMount, onDestroy } from 'svelte';

  const log = logger('script-editor', 'coral');
  let { actions, inputs } = baseData();

  export let source: {
    // TODO figure out the format for this
    simulations: unknown[];
    script: string;
  } | null;

  export let compiled: {
    timeout: number | undefined;
    script: string;
  } | null;

  export let taskTriggers: Record<string, TaskTrigger>;
  export let taskActions: Record<string, TaskAction>;

  let bundler: Bundler | null;

  function getBundler() {
    if (bundler) {
      return bundler;
    }

    bundler = new Bundler();
    return bundler;
  }

  onDestroy(() => {
    bundler?.destroy();
    bundler = null;
  });

  $: scriptTypeDefs = scriptTypeDefinitions({
    taskTriggers,
    taskActions,
    actions: $actions,
    inputs: $inputs,
  });

  $: log('generated script type definitions', scriptTypeDefs);

  let view: EditorView;
  export async function getState() {
    // TODO Extra lint checks and validation once those are in place.
    let s = view.state.doc.toString();

    let bundle = await getBundler().bundle({
      production: true,
      files: {
        'index.ts': s,
      },
    });

    if (bundle.error) {
      throw bundle.error;
    }

    return {
      source: {
        type: 'Js',
        data: {
          simulations: [],
          script: s,
        },
      },
      compiled: {
        type: 'Js',
        data: {
          timeout: undefined,
          script: bundle.code,
          map: JSON.stringify(bundle.map),
        },
      },
    };
  }

  let currentScript = source?.script ?? compiled?.script ?? '';
</script>

<div class="flex flex-col space-y-4">
  <Editor
    format="ts"
    bind:view
    on:change={({ detail: newScript }) => (currentScript = newScript)}
    notifyOnChange={true}
    contents={source?.script ?? compiled?.script ?? ''}
    tsDefs={{ 'TaskScript.d.ts': scriptTypeDefs }}
  />
  <ScriptSimulator {getBundler} context={{}} script={currentScript} />
</div>
