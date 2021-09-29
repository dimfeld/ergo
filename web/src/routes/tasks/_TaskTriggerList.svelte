<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { TaskTriggerInput } from '../../api_types';
  import { baseData } from '../../data';
  import sorter from 'sorters';
  import InlineEditTextField from '../../components/InlineEditTextField.svelte';

  const dispatch = createEventDispatcher<{ change: void }>();
  const notify = () => dispatch('change');

  export let triggers: Record<string, TaskTriggerInput>;

  function updateKey(oldKey: string, newKey: string) {
    triggers[newKey] = triggers[oldKey];
    delete triggers[oldKey];
    notify();
  }

  const { inputs } = baseData();
  $: triggerRows = Object.entries(triggers).map(([localId, trigger]) => {
    return {
      localId,
      trigger,
      input: $inputs.get(trigger.input_id),
    };
  });

  $: inputTypes = Array.from($inputs.values())
    .map((input) => ({ id: input.input_id, name: input.name }))
    .sort(sorter('name'));
</script>

<div id="task-triggers">
  <span class="header">Trigger ID</span>
  <span class="header">Trigger Name</span>
  <span class="header">Input Type</span>
  {#each triggerRows as trigger (trigger.localId)}
    <div class="pr-4">
      <InlineEditTextField
        value={trigger.localId}
        on:change={({ detail: newValue }) => updateKey(trigger.localId, newValue)}
      />
    </div>
    <div class="pr-4">
      <InlineEditTextField bind:value={trigger.trigger.name} on:change={notify} />
    </div>
    <select bind:value={trigger.trigger.input_id} on:change={notify}>
      {#each inputTypes as { id, name } (id)}
        <option value={id}>{name}</option>
      {/each}
    </select>
  {/each}
</div>

<style lang="postcss">
  #task-triggers {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: auto;
    align-items: center;
  }

  .header {
    @apply font-medium text-gray-800 dark:text-gray-200 pb-2;
  }
</style>
