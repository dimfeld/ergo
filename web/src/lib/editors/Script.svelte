<script lang="ts">
  import { EditorView } from '@codemirror/view';
  import { TaskAction, TaskTrigger } from '../api_types';
  import { baseData } from '../data';
  import { scriptTypeDefinitions } from './types/task_script_definitions';
  import { logger } from '../logger';
  import Editor from './Editor.svelte';
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
  onMount(() => {
    bundler = new Bundler();
    return () => {
      bundler.destroy();
      bundler = null;
    };
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

    let bundle = await bundler!.bundle({
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
</script>

<Editor
  format="ts"
  bind:view
  contents={source?.script ?? compiled?.script ?? ''}
  tsDefs={{ 'TaskScript.d.ts': scriptTypeDefs }}
/>
