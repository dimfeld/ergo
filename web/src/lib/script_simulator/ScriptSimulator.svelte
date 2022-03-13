<script lang="ts">
  import { browser } from '$app/env';
  import { Bundler } from '$lib/bundler';
  import Button from '$lib/components/Button.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Labelled from '$lib/components/Labelled.svelte';
  import Editor from '$lib/editors/Editor.svelte';
  import { formatJson } from '$lib/editors/format';
  import { onDestroy } from 'svelte';
  import { ConsoleMessage, RunOutput, SandboxWorker, sandboxWorker } from './messages';

  export let script: string;
  export let context: object;

  export let autosaveContext = true;
  export let getBundler: () => Bundler;

  interface RunRecord {
    output: RunOutput;
    input: {
      context: object;
      payload: object;
    };
  }

  let runOutputs: RunRecord[] = [];

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

    // TODO configuration of payload based on the available task triggers.
    let payload = {
      trigger: '',
    };

    let output = await sandbox.runScript({
      script: bundled.code,
      context,
      payload,
    });

    runOutputs = [
      {
        output,
        input: {
          payload,
          context,
        },
      },
      ...runOutputs,
    ];
  }
</script>

<div class="flex flex-col space-y-2">
  <div class="flex">
    <header class="label big-label mr-auto">Simulator</header>
  </div>

  <div class="h-56">
    <Editor
      format="json"
      contents={formatJson(context || {}, 'json')}
      bind:getContents={getContextContents}
    >
      <div slot="left-toolbar" class="flex space-x-4">
        <Button size="xs" on:click={run}>Run</Button>
        <Checkbox
          bind:value={autosaveContext}
          label="Automatically use output context on next run"
        />
      </div>
    </Editor>
  </div>

  <ol>
    {#each runOutputs as runOutput}
      <li>
        <div class="flex w-full space-x-2">
          <div class="flex-1">
            <Labelled label="Input">
              <pre class="max-h-64 overflow-auto">{JSON.stringify(runOutput.input.payload)}</pre>
            </Labelled>
          </div>

          <div class="flex-1">
            <Labelled label="Actions">
              <ul>
                {#each runOutput.output.actions as action}
                  <li>{JSON.stringify(action)}</li>
                {:else}
                  <li>None</li>
                {/each}
              </ul>
            </Labelled>
          </div>
        </div>

        <div class="flex w-full space-x-2">
          <div class="flex-1">
            Context Before Run:
            <pre>{JSON.stringify(runOutput.input.context, null, 2)}</pre>
          </div>
          <div class="flex-1">
            Context After Run:
            <pre>{JSON.stringify(runOutput.output.context, null, 2)}</pre>
          </div>
        </div>
      </li>
    {/each}
  </ol>
</div>
