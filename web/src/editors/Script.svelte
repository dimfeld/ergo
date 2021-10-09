<script lang="ts">
  import { EditorView } from '@codemirror/view';

  import Editor from './Editor.svelte';

  export let source: {} | null;

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
        },
      },
      compiled: {
        type: 'Js',
        data: {
          timeout: undefined,
          script: s,
        },
      },
    };
  }
</script>

<Editor format="js" bind:view contents={compiled?.script ?? ''} />
