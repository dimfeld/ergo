<script lang="ts">
  import { getStores } from '$app/stores';
  import Query from '^/components/Query.svelte';
  import TextField from '^/components/TextField.svelte';
  import type { TaskResult } from '^/api_types';
  import { getHeaderTextStore } from '^/header';

  import ScriptEditor from '^/editors/Script.svelte';
  import StateMachineEditor from '^/editors/StateMachine.svelte';
  import { fetchOnceQuery } from '^/api';

  const taskEditors = {
    Script: ScriptEditor,
    StateMachine: StateMachineEditor,
  };

  const { page } = getStores();

  $: taskQuery = fetchOnceQuery<TaskResult>(['tasks', $page.params.task_id]);
  $: task = $taskQuery.isSuccess ? $taskQuery.data : null;

  const headerText = getHeaderTextStore();
  $: if (task) {
    headerText.set(['Tasks', task.name]);
  }

  function initializeSource(type: string) {
    if (task) {
      task.source = { type, data: null };
    }
  }

  $: taskSource = task?.source || task?.compiled;
</script>

<Query query={taskQuery}>
  <section
    class="flex flex-col space-y-2 w-full p-2 rounded border border-gray-200 dark:border-gray-400 shadow-md"
  >
    <div class="flex w-full justify-between">
      <p class="text-sm">ID: {task.task_id}</p>
      <p>
        <span class="font-medium text-sm text-gray-700 dark:text-gray-300">Alias</span>
        <TextField type="text" bind:value={task.alias} placeholder="None" class="ml-2" />
      </p>
    </div>
    <p>Description: {task.description || ''}</p>
    <p>Modified {task.modified}</p>
    <p>Actions {JSON.stringify(task.actions)}</p>
    <p>Triggers {JSON.stringify(task.triggers)}</p>
  </section>

  <section class="mt-4">
    {#if taskSource}
      <svelte:component this={taskEditors[taskSource.type]} data={taskSource.data} />
    {:else}
      <p>Select a task type</p>
      <p class="flex space-x-2">
        <button on:click={() => initializeSource('StateMachine')}>State Machine</button>
        <button on:click={() => initializeSource('Script')}>Script</button>
      </p>
    {/if}
  </section>
</Query>
