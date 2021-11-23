<script lang="ts">
  import { PeriodicTaskTrigger, TaskTrigger } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import DangerButton from '$lib/components/DangerButton.svelte';
  import PlusIcon from '$lib/components/icons/Plus.svelte';
  import initWasm from '$lib/wasm';
  import * as dateFns from 'date-fns';
  import { parse_schedule, new_periodic_trigger_id } from 'ergo-wasm';

  export let trigger: TaskTrigger;

  let wasmLoaded = false;
  initWasm().then(() => {
    newItem = defaultNewItem();
    wasmLoaded = true;
  });

  function defaultNewItem(): PeriodicTaskTrigger {
    return {
      periodic_trigger_id: new_periodic_trigger_id(),
      name: '',
      payload: {},
      enabled: true,
      schedule: { type: 'Cron', data: '' },
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
          date: 'Never',
          time: '',
        };
      }

      let d = new Date(next);
      let date = dateFns.formatISO9075(d, { representation: 'date' });
      let time = dateFns.formatISO9075(d, { representation: 'time' });
      return { valid: true, date, time };
    } catch (e) {
      return { valid: false, date: 'Invalid Cron Pattern', time: '' };
    }
  }
</script>

{#if wasmLoaded}
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
    {#each trigger.periodic ?? [] as periodic, i}
      <li class="periodic-row">
        <input type="text" bind:value={periodic.name} placeholder="Schedule Name" />

        <input type="text" bind:value={periodic.schedule.data} placeholder="Schedule" />

        <div class="flex flex-col">
          <p class="text-sm leading-4">{nextCron(periodic.schedule.data).date}</p>
          <p class="text-sm leading-4">{nextCron(periodic.schedule.data).time}</p>
        </div>

        <DangerButton on:click={() => deleteIndex(i)}>
          <span slot="title"
            >Delete Trigger <span class="text-gray-700 dark:text-gray-200 font-bold"
              >{periodic.name}</span
            ></span
          >
        </DangerButton>
      </li>
    {:else}
      <li>No active schedules</li>
    {/each}
  </ul>
  <Button class="mt-2" on:click={addItem}>Add Schedule</Button>
{/if}

<style>
  .periodic-row {
    display: grid;
    grid-template-rows: auto;
    grid-template-columns: repeat(2, 1fr) 8rem 2rem;
    column-gap: 1em;
  }

  header.periodic-row {
    align-items: end;
  }

  li.periodic-row {
    align-items: center;
  }
</style>
