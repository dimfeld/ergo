<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { TaskActionInput } from '../../api_types';
  import { baseData } from '../../data';
  import sorter from 'sorters';
  import InlineEditTextField from '../../components/InlineEditTextField.svelte';

  const dispatch = createEventDispatcher<{ change: void }>();
  const notify = () => dispatch('change');

  export let taskActions: Record<string, TaskActionInput>;

  function updateKey(oldKey: string, newKey: string) {
    taskActions[newKey] = taskActions[oldKey];
    delete taskActions[oldKey];
    notify();
  }

  const { actions } = baseData();
  $: actionRows = Object.entries(taskActions).map(([localId, action]) => {
    return {
      localId,
      taskAction: action,
      action: $actions.get(action.action_id),
    };
  });

  $: actionTypes = Array.from($actions.values())
    .map((action) => ({ id: action.action_id, name: action.name }))
    .sort(sorter('name'));
</script>

<div id="task-actions">
  <span class="header">Local ID</span>
  <span class="header">Local Action Name</span>
  <span class="header">Action Type</span>
  {#each actionRows as action (action.localId)}
    <div class="pr-4">
      <InlineEditTextField
        value={action.localId}
        on:change={({ detail: newValue }) => updateKey(action.localId, newValue)}
      />
    </div>
    <div class="pr-4">
      <InlineEditTextField bind:value={action.taskAction.name} on:change={notify} />
    </div>
    <select bind:value={action.taskAction.action_id} on:change={notify}>
      {#each actionTypes as { id, name } (id)}
        <option value={id}>{name}</option>
      {/each}
    </select>
  {/each}
</div>

<style lang="postcss">
  #task-actions {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: auto;
    align-items: center;
  }

  .header {
    @apply font-medium text-gray-800 dark:text-gray-200 pb-2;
  }
</style>
