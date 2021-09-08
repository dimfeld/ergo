<script lang="ts">
  import { getStores } from '$app/stores';
  import Query from '^/components/Query.svelte';
  import TextField from '^/components/TextField.svelte';
  import type { TaskResult } from '^/api_types';
  import { getHeaderTextStore } from '^/header';

  import ScriptEditor from '^/editors/Script.svelte';
  import StateMachineEditor from '^/editors/StateMachine.svelte';
  import { fetchOnceQuery, objectEditor } from '^/api';

  const taskEditors = {
    Script: ScriptEditor,
    StateMachine: StateMachineEditor,
  };

  const { page } = getStores();

  function defaultTask(): TaskResult {
    let created = new Date().toISOString();
    return {
      task_id: '',
      name: '',
      source: null,
      compiled: null,
      actions: {},
      triggers: {},
      state: null,
      enabled: false,
      modified: created,
      created,
      task_template_version: 0,
      alias: null,
      description: null,
    };
  }

  $: newTask = $page.params.task_id === 'new';
  $: taskQuery = newTask ? null : fetchOnceQuery<TaskResult>(['tasks', $page.params.task_id]);
  $: task = objectEditor(taskQuery, defaultTask);

  const headerText = getHeaderTextStore();
  $: if ($task) {
    headerText.set(['Tasks', $task.name || 'New Task']);
  }

  function initializeSource(type: string) {
    if ($task) {
      $task.source = { type, data: null };
    }
  }

  $: taskSource = $task?.source || $task?.compiled;
</script>

<div class="flex flex-col flex-grow">
  <Query query={taskQuery}>
    <section
      class="flex flex-col space-y-2 w-full p-2 rounded border border-gray-200 dark:border-gray-400 shadow-md"
    >
      <div class="flex w-full justify-between">
        <p class="text-sm">
          ID: <span class:text-gray-500={!$task.task_id}>{$task.task_id || 'New Task'}</span>
        </p>
        <p>
          <span class="font-medium text-sm text-gray-700 dark:text-gray-300">Alias</span>
          <TextField type="text" bind:value={$task.alias} placeholder="None" class="ml-2" />
        </p>
      </div>
      <p>Description: {$task.description || ''}</p>
      <p>Modified {$task.modified}</p>
      <p>Actions {JSON.stringify($task.actions)}</p>
      <p>Triggers {JSON.stringify($task.triggers)}</p>
    </section>

    <section class="flex flex-col flex-grow mt-4">
      {#if taskSource}
        <div class="flex-grow grid grid-rows-1 grid-cols-1 place-items-stretch">
          <svelte:component
            this={taskEditors[taskSource.type]}
            source={$task?.source.data}
            compiled={$task?.compiled?.data}
          />
        </div>
      {/if}
      <p>
        {#if taskSource}Change the task type{:else}Select a task type{/if}
      </p>
      <p class="flex space-x-2">
        <button on:click={() => initializeSource('StateMachine')}>State Machine</button>
        <button on:click={() => initializeSource('Script')}>Script</button>
        <button on:click={() => initializeSource('Flowchart')}>FlowChart</button>
      </p>
    </section>
  </Query>
</div>
