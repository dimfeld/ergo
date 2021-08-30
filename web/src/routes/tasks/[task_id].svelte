<script lang="ts">
  import { useQuery } from '@sveltestack/svelte-query';
  import { getContext } from 'svelte';
  import { getStores } from '$app/stores';
  import Loading from '^/components/Loading.svelte';
  import type { TaskResult } from '^/api_types';
  import { getHeaderTextStore } from '^/header';

  const headerText = getHeaderTextStore();

  const { page } = getStores();
  $: taskQuery = useQuery<TaskResult>(['tasks', $page.params.task_id]);
  $: task = $taskQuery.isSuccess ? $taskQuery.data : null;

  $: if (task) {
    headerText.set(task.name);
  }
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
    <p>Task {JSON.stringify(task)}</p>
  </section>
{/if}
