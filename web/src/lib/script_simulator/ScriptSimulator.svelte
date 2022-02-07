<script lang="ts">
  import { browser } from '$app/env';
  import { Bundler } from '$lib/bundler';
  import Button from '$lib/components/Button.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Editor from '$lib/editors/Editor.svelte';
  import { formatJson } from '$lib/editors/format';
  import iFrameContents from './iframe.html?raw';
  import { RunOutput } from './messages';

  export let script: string;
  export let context: object;

  export let autosaveContext = true;
  export let getBundler: () => Bundler;

  let runOutputs: RunOutput[] = [];
</script>

<Button>Run</Button>
<Checkbox bind:value={autosaveContext} label="Automatically use output context on next run" />

<Editor format="json" contents={formatJson(context || {}, 'json')} />

<ol>
  {#each runOutputs as runOutput (runOutput.id)}
    <li>
      Actions:
      <ul>
        {#each runOutput.actions as action}
          <li>{JSON.stringify(action)}</li>
        {:else}
          <li>None</li>
        {/each}
      </ul>
      Context After Run:
      <pre>{JSON.stringify(runOutput.context, null, 2)}</pre>
    </li>
  {/each}
</ol>

<iframe
  class="absolute top-0 right-0 h-0 w-0"
  aria-hidden="true"
  title="Simulation Sandbox"
  sandbox="allow-scripts"
  srcdoc={browser ? iFrameContents : ''}
/>
