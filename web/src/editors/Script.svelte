<script lang="ts">
  import { EditorView } from '@codemirror/view';

  import Editor from './Editor.svelte';

  export let source: {
    timeout: number | undefined;
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
        timeout: undefined,
        script: s,
      },
      compiled: {
        timeout: undefined,
        script: s,
      },
    };
  }
</script>

<Editor format="js" bind:view contents={source?.script ?? ''} />
