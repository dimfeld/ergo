<script lang="ts">
  import { TaskActionInput } from '../../api_types';
  import { baseData } from '../../data';

  const { actions } = baseData();

  export let taskActions: Record<string, TaskActionInput>;

  $: actionRows = Object.entries(taskActions).map(([localId, action]) => {
    return {
      localId,
      taskAction: action,
      action: $actions.get(action.action_id),
    };
  });
</script>

<div id="task-actions">
  <span class="header">Local ID</span>
  <span class="header">Local Action Name</span>
  <span class="header">Action Name</span>
  {#each actionRows as action (action.localId)}
    <span>{action.localId}</span>
    <span>{action.taskAction.name}</span>
    <span>{action.action?.name}</span>
  {/each}
</div>

<style lang="postcss">
  #task-actions {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: auto;
  }

  .header {
    @apply font-medium text-gray-800 dark:text-gray-200;
  }
</style>
