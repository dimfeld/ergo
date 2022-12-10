<script lang="ts">
  import type { PeriodicTaskTrigger, TaskTrigger } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import DangerButton from '$lib/components/DangerButton.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import PlusIcon from '$lib/components/icons/Plus.svelte';
  import { defaultFromJsonSchema } from '$lib/json_schema';
  import { baseData } from '$lib/data';
  import initWasm from '$lib/wasm';
  import * as dateFns from 'date-fns';
  import { parse_schedule, new_periodic_trigger_id } from 'ergo-wasm';
  import { formatJson } from '$lib/editors/format';
  import cronstrue from 'cronstrue';
  import Editor from '$lib/editors/Editor.svelte';
  import { showTippy } from '$lib/components/tippy';

  export let trigger: TaskTrigger;

  const { inputs } = baseData();

  let wasmLoaded = false;
  initWasm().then(() => {
    newItem = defaultNewItem();
    wasmLoaded = true;
  });

  function defaultNewItem(): PeriodicTaskTrigger {
    return {
      periodic_trigger_id: new_periodic_trigger_id(),
      name: '',
      payload: defaultFromJsonSchema($inputs.get(trigger.input_id)?.payload_schema),
      enabled: true,
      // Default to every hour since that's convenient
      schedule: { type: 'Cron', data: '0 0 * * * *' },
    };
  }

  let newItem: ReturnType<typeof defaultNewItem>;

  async function deleteIndex(i: number) {
    if (!trigger.periodic) {
      return;
    }

    trigger.periodic = [...trigger.periodic.slice(0, i), ...trigger.periodic.slice(i + 1)];
  }

  function addItem() {
    trigger.periodic = [...(trigger.periodic || []), defaultNewItem()];
  }

  function nextCron(schedule: string) {
    if (!schedule) {
      return { valid: false, date: '', time: '' };
    }

    try {
      let next = parse_schedule(schedule);
      if (!next) {
        return {
          valid: false,
          time: 'Never',
        };
      }

      let d = new Date(next);
      let time = dateFns.formatISO9075(d, { representation: 'complete' });
      return { valid: true, desc: cronstrue.toString(schedule), time };
    } catch (e) {
      return { valid: false, date: 'Invalid Cron Pattern', time: '' };
    }
  }

  function parsePayloadValue(periodic: PeriodicTaskTrigger, value: string) {
    try {
      periodic.payload = JSON.parse(value);
    } catch (e) {}
  }
</script>

{#if wasmLoaded}
  <header class="periodic-row mt-2 text-sm font-medium">
    <span>Name</span>
    <div class="pl-1">
      <p>Schedule</p>
      <p>S M H D M DOW [Year]</p>
    </div>

    <span>Will Run</span>
    <span />
  </header>
  <ul class="mt-2 flex flex-col space-y-2">
    {#each trigger.periodic ?? [] as periodic, i}
      {@const next = nextCron(periodic.schedule.data)}
      <li class="periodic-row">
        <!-- TODO Make this into a "name, next run" pair that expands into the rest -->
        <input type="text" bind:value={periodic.name} placeholder="Schedule Name" />

        <input type="text" bind:value={periodic.schedule.data} placeholder="Schedule" />

        <div class="flex flex-col">
          {#if next.valid}
            <div class="text-xs leading-4">
              {next.desc}
              <Tooltip class="text-sm font-medium">
                Next Run: {next.time}
              </Tooltip>
            </div>
          {:else}
            <p class="text-xs">Invalid Pattern</p>
          {/if}
        </div>

        <div class="flex space-x-2">
          <Dropdown closeOnClickInside={false} pad={false}>
            <svelte:fragment slot="button">
              <Button class="w-8" title="Payload" iconButton>[]</Button>
            </svelte:fragment>
            <div class="h-64 w-64 p-0.5">
              <Editor
                format="json"
                toolbar={false}
                contents={formatJson(periodic.payload, 'json')}
                on:change={({ detail: text }) => parsePayloadValue(periodic, text)}
              />
            </div>
          </Dropdown>

          <DangerButton on:click={() => deleteIndex(i)}>
            <span slot="title"
              >Delete Trigger <span class="font-bold text-gray-700 dark:text-gray-200"
                >{periodic.name}</span
              ></span
            >
          </DangerButton>
        </div>
      </li>
    {:else}
      <li>No active schedules</li>
    {/each}
  </ul>
  <Button class="mt-2" on:click={addItem}>Add New Schedule</Button>
{/if}

<style>
  .periodic-row {
    display: grid;
    grid-template-rows: auto;
    grid-template-columns: repeat(2, 1fr) 8rem 4.5rem;
    column-gap: 1em;
  }

  header.periodic-row {
    align-items: end;
  }

  li.periodic-row {
    align-items: center;
  }
</style>
