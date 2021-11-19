<script lang="ts">
  import { EditorView } from '@codemirror/view';
  import { TaskAction, TaskTrigger } from '../api_types';
  import { baseData } from '../data';
  import { scriptTypeDefinitions } from './types/task_script_definitions';

  import Editor from './Editor.svelte';

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

  $: scriptTypeDefs = scriptTypeDefinitions({
    taskTriggers,
    taskActions,
    actions: $actions,
    inputs: $inputs,
  });

  let view: EditorView;
  export function getState() {
    // TODO Extra lint checks and validation once those are in place.
    let s = view.state.doc.toString();
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
          // TODO Compile TS down to JS for "compiled"
          script: s,
        },
      },
    };
  }
</script>

<Editor
  format="js"
  bind:view
  contents={compiled?.script ?? source?.script ?? ''}
  tsDefs={{ 'TaskScript.d.ts': scriptTypeDefs }}
/>
