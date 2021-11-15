<script lang="ts">
  import { TaskTrigger } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import DangerButton from '$lib/components/DangerButton.svelte';
  import PlusIcon from '$lib/components/icons/Plus.svelte';
  import InlineEditTextField from '$lib/components/InlineEditTextField.svelte';
  import initWasm from '$lib/wasm';
  import { parse_schedule } from 'ergo-wasm';

  export let trigger: TaskTrigger;

  let wasmLoaded = false;
  initWasm().then(() => (wasmLoaded = true));

  function defaultNewItem() {
    return {
      isNewItem: true,
      periodic: {
        name: '',
        payload: {},
        enabled: true,
        schedule: { type: 'Cron', data: '' },
      },
    };
  }

  let newItem = defaultNewItem();
  $: periodic = [
    ...(trigger.periodic ?? []).map((periodic) => ({ periodic, isNewItem: false })),
    newItem,
  ];

  function deleteIndex(i) {
    trigger.periodic = [...trigger.periodic.slice(0, i), ...trigger.periodic.slice(i + 1)];
  }

  function addItem(p) {
    trigger.periodic = [...(trigger.periodic || []), p];

    newItem = defaultNewItem();
  }

  function nextCron(schedule: string) {
    if (!schedule) {
      return { valid: false, text: '' };
    }

    try {
      let next = parse_schedule(schedule);
      return { valid: Boolean(next), text: next ?? 'Never' };
    } catch (e) {
      console.error(e);
      let err = e as Error;
      return { valid: false, text: err.message ?? err };
    }
  }

  let parsed = new WeakMap<object, string>();
</script>

{#if wasmLoaded}
  <div class="max-h-32 w-[48rem] py-2 px-4">
    <p class="text-lg">Periodic Triggers</p>
    <header class="periodic-row mt-2 text-sm font-medium">
      <span>Name</span>
      <div>
        <p>Schedule</p>
        <p>S M H D M DOW [Year]</p>
      </div>

      <span>Next Run</span>
      <span />
    </header>
    <ul class="flex flex-col mt-2 space-y-2">
      {#each periodic as { periodic, isNewItem }, i}
        <li class="periodic-row">
          <InlineEditTextField
            bind:value={periodic.name}
            placeholder={isNewItem ? 'New Schedule Name' : ''}
          />

          <InlineEditTextField
            bind:value={periodic.schedule.data}
            on:input={({ detail }) => {
              parsed.set(periodic, nextCron(detail).text);
              parsed = parsed;
            }}
            placeholder="Schedule"
          />

          <span class="text-sm"
            >{parsed.get(periodic) ?? nextCron(periodic.schedule.data).text}</span
          >

          {#if isNewItem}
            <Button
              disabled={!periodic.schedule.data}
              on:click={() => addItem(periodic)}
              iconButton={true}
            >
              <PlusIcon />
            </Button>
          {:else}
            <DangerButton on:click={() => deleteIndex(i)}>
              <span slot="title"
                >Delete Trigger <span class="text-gray-700 dark:text-gray-200 font-bold"
                  >{periodic.name}</span
                ></span
              >
            </DangerButton>
          {/if}
        </li>
      {/each}
    </ul>
  </div>
{/if}

<style>
  .periodic-row {
    display: grid;
    grid-template-rows: auto;
    grid-template-columns: repeat(2, 1fr) 16rem 2rem;
    column-gap: 1em;
  }

  header.periodic-row {
    align-items: end;
  }

  li.periodic-row {
    align-items: center;
  }
</style>
