<script lang="ts">
  import { useQuery } from '@sveltestack/svelte-query';
  import { getStores } from '$app/stores';
  import Loading from '^/components/Loading.svelte';
  import type { TaskResult } from '^/api_types';
  import { fetchOnceQuery } from '^/api';
  import { getHeaderTextStore } from '^/header';

  import ScriptEditor from '^/editors/Script.svelte';
  import StateMachineEditor from '^/editors/StateMachine.svelte';

  const taskEditors = {
    Script: ScriptEditor,
    StateMachine: StateMachineEditor,
  };

  const headerText = getHeaderTextStore();

  const { page } = getStores();
  $: taskQuery = fetchOnceQuery<TaskResult>(['tasks', $page.params.task_id]);
  $: task = $taskQuery.isSuccess ? $taskQuery.data : null;

  $: if (task) {
    headerText.set(task.name);
  }

  $: taskSource = task?.source || task?.compiled;
</script>

{#if $taskQuery.isLoading}
  <Loading />
{:else if task}
  <section
    class="flex flex-col space-y-2 w-full p-2 rounded border border-gray-200 dark:border-gray-400 shadow-md"
  >
    <div class="flex w-full justify-between">
      <p>ID: {task.task_id}</p>
      <p>Alias: {task.alias || 'None'}</p>
    </div>
    <p>Description: {task.description || ''}</p>
    <p>Modified {task.modified}</p>
    <p>Actions {JSON.stringify(task.actions)}</p>
    <p>Triggers {JSON.stringify(task.triggers)}</p>
  </section>
  <section class="mt-4">
    <svelte:component this={taskEditors[taskSource.type]} data={taskSource.data} />
  </section>
{/if}
