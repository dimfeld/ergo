<script context="module" lang="ts">
  export interface TaskActionEditorData {
    taskActionId: string;
    action: TaskAction;
  }
</script>

<script lang="ts">
  import type { TaskAction } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import Labelled from '$lib/components/Labelled.svelte';
  import type { ModalCloser } from '$lib/components/Modal.svelte';
  import { baseData } from '$lib/data';
  import sorter from 'sorters';

  export let allActions: Record<string, TaskAction>;
  export let taskActionId: string;
  export let action: TaskAction;
  export let close: ModalCloser<TaskActionEditorData>;

  let existingActionId = taskActionId;
  const { actions, accounts } = baseData();

  $: actionTypes = Array.from($actions.values())
    .map((action) => ({ id: action.action_id, name: action.name }))
    .sort(sorter('name'));

  function validate() {
    if (taskActionId === existingActionId) {
      return;
    }

    if (!taskActionId) {
      return 'ID is required';
    }

    if (taskActionId in allActions) {
      return 'IDs must be unique';
    }
  }

  let errorMessage: string | undefined = '';
  function handleSubmit() {
    validate();
    if (!errorMessage) {
      close({ taskActionId, action });
    }
  }

  $: selectedAction = $actions.get(action.action_id);
  $: allowedAccountTypes = new Set(selectedAction?.account_types || []);
  $: accountOptions = Array.from($accounts.values()).filter((a) =>
    allowedAccountTypes.has(a.account_type_id)
  );
</script>

<form on:submit|preventDefault={handleSubmit} class="flex flex-col space-y-4">
  <div class="flex space-x-4">
    <Labelled class="flex-1" label="Local ID">
      <input type="text" bind:value={taskActionId} />
    </Labelled>
    <Labelled class="flex-1" label="Action Type">
      <select bind:value={action.action_id}>
        {#each actionTypes as { id, name } (id)}
          <option value={id}>{name}</option>
        {/each}
      </select>
    </Labelled>
  </div>
  <Labelled label="Description">
    <input type="text" class="w-full" bind:value={action.name} />
  </Labelled>
  {#if selectedAction?.account_types?.length && accountOptions.length}
    <Labelled label="Account">
      <select class="w-full" bind:value={action.account_id}>
        {#if !selectedAction?.account_required}
          <option value={null}>None</option>
        {/if}
        {#each accountOptions as account}
          <option value={account.account_id}>{account.name}</option>
        {/each}
      </select>
    </Labelled>
  {/if}
  <!--TODO Action Template Editor -->
  <div class="flex items-center justify-end space-x-2">
    <span class="flex-1 text-red-500">{errorMessage ?? ''}</span>
    <Button style="primary" type="submit">OK</Button>
    <Button on:click={() => close()}>Cancel</Button>
  </div>
</form>
