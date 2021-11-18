<script lang="ts">
  import { EditorView } from '@codemirror/view';

  import Editor from './Editor.svelte';
  import ergoTypeDefs from './types/TaskScript.d.ts?raw';

  export let source: {
    // TODO figure out the format for this
    simulations: unknown[];
    script: string;
  } | null;

  export let compiled: {
    timeout: number | undefined;
    script: string;
  } | null;

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
  tsDefs={{ 'TaskScript.d.ts': ergoTypeDefs }}
/>
