<script context="module" lang="ts">
  export interface TaskTriggerEditorData {
    triggerId: string;
    trigger: TaskTrigger;
  }
</script>

<script lang="ts">
  import type { TaskTrigger } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import Labelled from '$lib/components/Labelled.svelte';
  import type { ModalCloser } from '$lib/components/Modal.svelte';
  import { baseData } from '$lib/data';
  import sorter from 'sorters';
  import PeriodicTriggerEditor from './_PeriodicTriggerEditor.svelte';

  export let triggerId: string;
  export let trigger: TaskTrigger;
  export let allTriggers: Record<string, TaskTrigger>;
  export let close: ModalCloser<TaskTriggerEditorData>;

  let existingTriggerId = triggerId;

  let errorMessage: string | undefined = '';

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

  const { inputs } = baseData();
  $: inputTypes = Array.from($inputs.values())
    .map((input) => ({ id: input.input_id, name: input.name }))
    .sort(sorter('name'));
</script>

<form class="w-[40rem] flex flex-col space-y-4" on:submit|preventDefault={handleSubmit}>
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
  <div class="flex space-x-2 items-center justify-end">
    <span class="flex-1 text-red-500">{errorMessage ?? ''}</span>
    <Button style="primary" type="submit">OK</Button>
    <Button on:click={() => close()}>Cancel</Button>
  </div>
</form>
