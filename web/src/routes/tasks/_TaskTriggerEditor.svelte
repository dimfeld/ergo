<script context="module" lang="ts">
  export interface TaskTriggerEditorData {
    triggerId: string;
    trigger: TaskTrigger;
  }
</script>

<script lang="ts">
  import { TaskTrigger } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import Labelled from '$lib/components/Labelled.svelte';
  import { ModalCloser } from '$lib/components/Modal.svelte';
  import { baseData } from '$lib/data';
  import sorter from 'sorters';
  import initWasm from '$lib/wasm';
  import { parse_schedule, new_periodic_trigger_id } from 'ergo-wasm';
  import Plus from '$lib/components/icons/Plus.svelte';
  import PeriodicTriggerEditor from './_PeriodicTriggerEditor.svelte';

  export let triggerId: string;
  export let trigger: TaskTrigger;
  export let allTriggers: Record<string, TaskTrigger>;
  export let close: ModalCloser<TaskTriggerEditorData>;

  let existingTriggerId = triggerId;

  let errorMessage: string | undefined = '';

  let wasmLoaded = false;
  initWasm().then(() => (wasmLoaded = true));

  function validateId() {
    if (triggerId === existingTriggerId) {
      return;
    }

    if (!triggerId) {
      return 'ID is required';
    }

    if (triggerId in allTriggers) {
      return 'IDs must be unique';
    }
  }

  function validate() {
    errorMessage = validateId();
  }

  function handleSubmit() {
    validate();
    if (!errorMessage) {
      close({ triggerId, trigger });
    }
  }

  function nextCron(schedule: string) {
    if (!schedule) {
      return { valid: false, text: '' };
    }

    try {
      let next = parse_schedule(schedule);
      return { valid: Boolean(next), text: next ?? 'Never' };
    } catch (e) {
      return { valid: false, text: 'Invalid Cron Pattern' };
    }
  }

  const { inputs } = baseData();
  $: inputTypes = Array.from($inputs.values())
    .map((input) => ({ id: input.input_id, name: input.name }))
    .sort(sorter('name'));
</script>

<form class="w-full max-w-xl flex flex-col space-y-4" on:submit|preventDefault={handleSubmit}>
  <div class="w-full flex space-x-4">
    <Labelled class="flex-1" label="Local ID"
      ><input class="w-full" type="text" bind:value={triggerId} /></Labelled
    >
    <Labelled class="flex-1" label="Input Type">
      <select class="w-full" bind:value={trigger.input_id}>
        {#each inputTypes as { id, name } (id)}
          <option value={id}>{name}</option>
        {/each}
      </select>
    </Labelled>
  </div>
  <Labelled label="Description"
    ><input class="w-full" type="text" bind:value={trigger.name} /></Labelled
  >
  <Labelled label="Schedules">
    <PeriodicTriggerEditor {trigger} />
  </Labelled>
  <div class="flex space-x-4 items-center justify-end">
    <span class="flex-1 text-red-500">{errorMessage ?? ''}</span>
    <Button type="submit">OK</Button>
    <Button on:click={() => close()}>Cancel</Button>
  </div>
</form>

<style lang="postcss">
  .periodic-row {
    display: grid;
    grid-template-rows: auto;
    grid-template-columns: minmax(8rem, 1fr) minmax(12rem, 1fr) 16rem auto;
    column-gap: 1em;
  }

  header.periodic-row {
    align-items: end;
  }
</style>
