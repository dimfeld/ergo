<script lang="ts">
  import { TaskTriggerInput } from '../../api_types';
  import { baseData } from '../../data';

  const { inputs, actions } = baseData();

  export let triggers: Record<string, TaskTriggerInput>;

  $: triggerRows = Object.entries(triggers).map(([localId, trigger]) => {
    return {
      localId,
      trigger,
      input: $inputs.get(trigger.input_id),
    };
  });
</script>

<div id="task-triggers">
  <span class="header">Local ID</span>
  <span class="header">Local Trigger Name</span>
  <span class="header">Input Name</span>
  {#each triggerRows as trigger (trigger.localId)}
    <span>{trigger.localId}</span>
    <span>{trigger.trigger.name}</span>
    <span>{trigger.input?.name}</span>
  {/each}
</div>

<style lang="postcss">
  #task-triggers {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: auto;
  }

  .header {
    @apply font-medium text-gray-800 dark:text-gray-200;
  }
</style>
