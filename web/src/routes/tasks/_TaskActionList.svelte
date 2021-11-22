<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { TaskActionInput } from '$lib/api_types';
  import { baseData } from '$lib/data';
  import sorter from 'sorters';
  import Button from '$lib/components/Button.svelte';
  import DangerButton from '$lib/components/DangerButton.svelte';
  import InlineEditTextField from '$lib/components/InlineEditTextField.svelte';
  import PlusIcon from '$lib/components/icons/Plus.svelte';

  const dispatch = createEventDispatcher<{ change: void }>();
  const notify = () => dispatch('change');

  export let taskId: string;
  export let taskActions: Record<string, TaskActionInput>;

  function noop() {}

  function updateKey(oldKey: string, newKey: string) {
    taskActions[newKey] = taskActions[oldKey];
    delete taskActions[oldKey];
    notify();
  }

  function deleteKey(key: string) {
    delete taskActions[key];
    notify();
  }

  const { actions } = baseData();
  $: actionRows = [
    ...Object.entries(taskActions).map(([localId, action]) => {
      return {
        localId,
        taskAction: action,
        action: $actions.get(action.action_id),
        isNewItem: false,
      };
    }),
    newItem,
  ];

  function newItemTemplate() {
    let action = $actions.values().next().value;
    return {
      localId: '',
      taskAction: {
        action_id: action?.action_id,
        account_id: null,
        action_template: null,
        name: '',
        description: null,
      },
      action,
      isNewItem: true,
    };
  }

  let newItem = newItemTemplate();

  function addItem(action) {
    if (!action.localId) {
      return;
    }

    taskActions[action.localId] = {
      ...action.taskAction,
      task_local_id: action.localId,
      task_id: taskId,
    };

    newItem = newItemTemplate();
    notify();
  }

  $: actionTypes = Array.from($actions.values())
    .map((action) => ({ id: action.action_id, name: action.name }))
    .sort(sorter('name'));

  function validateId(value: string, existing: string) {
    if (value === existing) {
      return;
    }

    if (!value) {
      return 'ID is required';
    }

    if (value in actions) {
      return 'IDs must be unique';
    }
  }

  const getKeyChangeHandler = (action) =>
    action.isNewItem
      ? ({ detail: newValue }) => (newItem.localId = newValue)
      : ({ detail: newValue }) => updateKey(action.localId, newValue);
  const notifyHandler = (action) => (action.isNewItem ? noop : notify);
</script>

<div id="task-actions">
  <span class="header">Local ID</span>
  <span class="header">Description</span>
  <span class="header">Action Type</span>
  <span class="header" />
  {#each actionRows as action (action.localId)}
    <div class="pr-4">
      <InlineEditTextField
        value={action.localId}
        validate={validateId}
        placeholder={action.isNewItem ? 'New Action ID' : ''}
        on:change={getKeyChangeHandler(action)}
      />
    </div>
    <div class="pr-4">
      <InlineEditTextField
        bind:value={action.taskAction.name}
        placeholder={action.isNewItem ? 'New Action Name' : ''}
        on:change={notifyHandler(action)}
      />
    </div>
    <select bind:value={action.taskAction.action_id} on:change={notifyHandler(action)}>
      {#each actionTypes as { id, name } (id)}
        <option value={id}>{name}</option>
      {/each}
    </select>
    {#if action.isNewItem}
      <Button disabled={!action.localId} on:click={() => addItem(action)} iconButton={true}>
        <PlusIcon />
      </Button>
    {:else}
      <DangerButton on:click={() => deleteKey(action.localId)}>
        <span slot="title"
          >Delete Action <span class="text-gray-700 dark:text-gray-200 font-bold"
            >{action.taskAction.name || action.localId}</span
          ></span
        >
      </DangerButton>
    {/if}
  {/each}
</div>

<style lang="postcss">
  #task-actions {
    display: grid;
    grid-template-columns: repeat(3, 1fr) auto;
    grid-template-rows: auto;
    row-gap: 1em;
    column-gap: 1em;
    align-items: center;
  }

  .header {
    @apply font-medium text-gray-800 dark:text-gray-200;
  }
</style>
