<script lang="ts">
  import { browser } from '$app/env';
  import { Bundler } from '$lib/bundler';
  import Button from '$lib/components/Button.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Editor from '$lib/editors/Editor.svelte';
  import { formatJson } from '$lib/editors/format';
  import { onDestroy } from 'svelte';
  import { ConsoleMessage, RunOutput, SandboxWorker, sandboxWorker } from './messages';

  export let script: string;
  export let context: object;

  export let autosaveContext = true;
  export let getBundler: () => Bundler;

  let runOutputs: RunOutput[] = [];

  let sandbox: SandboxWorker | null = null;
  onDestroy(() => {
    sandbox?.destroy();
    sandbox = null;
  });

  let getContextContents: () => string;

  function handleConsole(message: ConsoleMessage) {
    // TODO Add console messages to a list
  }

  async function run() {
    let bundler = getBundler();
    let bundled = await bundler.bundle({
      files: {
        'index.ts': script,
      },
    });

    if (bundled.error) {
      throw bundled.error;
    }

    let context = JSON.parse(getContextContents());

    if (!sandbox) {
      sandbox = sandboxWorker({
        console: handleConsole,
      });
    }

    sandbox.runScript({
      script: bundled.code,
      context,
      // TODO configuration of payload based on the available task triggers.
      payload: {
        trigger: '',
      },
    });
  }
</script>

<Button on:click={run}>Run</Button>
<Checkbox bind:value={autosaveContext} label="Automatically use output context on next run" />

<Editor
  format="json"
  contents={formatJson(context || {}, 'json')}
  bind:getContents={getContextContents}
/>

<ol>
  {#each runOutputs as runOutput}
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
