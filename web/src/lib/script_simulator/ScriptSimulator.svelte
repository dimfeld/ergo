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

  let payload = { trigger: '' };

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
  let getPayloadContents: () => string;

  let consoleMessages: ConsoleMessage[] = [];
  function handleConsole(message: ConsoleMessage) {
    consoleMessages.push(message);
    consoleMessages = consoleMessages;
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
    let payload = JSON.parse(getPayloadContents());

    if (!sandbox) {
      sandbox = sandboxWorker({
        console: handleConsole,
      });
    }

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

  <div class="flex h-56 divide-x">
    <Editor
      class="flex-1 pr-4"
      format="json"
      contents={formatJson(payload || {}, 'json')}
      bind:getContents={getPayloadContents}
    >
      <div slot="left-toolbar" class="flex space-x-4">
        <Button size="xs" on:click={run}>Run</Button>
      </div>
    </Editor>
    <Editor
      class="flex-1 pl-4"
      format="json"
      contents={formatJson(context || {}, 'json')}
      bind:getContents={getContextContents}
    >
      <div slot="left-toolbar">
        <Checkbox
          bind:value={autosaveContext}
          label="Automatically replace context with run output"
        />
      </div>
    </Editor>
  </div>

  <div class="grid max-h-[64em] grid-cols-2 space-x-4">
    <Labelled class="min-h-0 min-w-0" label="Console">
      <ul class="min-h-0 min-w-0 overflow-auto">
        {#each consoleMessages as message}
          <li>{message.level}: {message.args}</li>
        {:else}
          <li>No console messages</li>
        {/each}
      </ul>
    </Labelled>
    <Labelled class="min-h-0 min-w-0" label="Results">
      <ol class="min-h-0 min-w-0 overflow-auto">
        {#each runOutputs as runOutput}
          <li>
            <div class="flex w-full space-x-2">
              <div class="flex-1">
                <Labelled label="Input">
                  <pre class="max-h-64 overflow-auto">{JSON.stringify(
                      runOutput.input.payload
                    )}</pre>
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
        {:else}
          <li>No results yet</li>
        {/each}
      </ol>
    </Labelled>
  </div>
</div>
