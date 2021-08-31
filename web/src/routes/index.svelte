<script lang="ts">
  import type { InputsLogEntry, TaskDescription } from '../api_types';
  import LogTimeline from '^/components/LogTimeline.svelte';
  import Loading from '^/components/Loading.svelte';
  import TaskRow from '^/components/TaskRow.svelte';
  import { useQuery } from '@sveltestack/svelte-query';
  import { getHeaderTextStore } from '^/header';
  import sorter from 'sorters';
  getHeaderTextStore().set('Dashboard');

  const recentLogs = useQuery<InputsLogEntry[]>('logs');
  const taskQuery = useQuery<TaskDescription[]>('tasks');

  $: tasks = ($taskQuery.data ?? [])
    .slice()
    .sort(sorter({ value: 'last_triggered', descending: true }));
</script>

<div class="flex">
  <section class="flex-grow min-w-max">
    {#if $taskQuery.isLoading}
      <Loading />
    {:else}
      {#each tasks as task (task.task_id)}
        <TaskRow {task} />
      {/each}
    {/if}
  </section>
  <section class="min-h-full ml-4">
    {#if $recentLogs.isLoading}
      <Loading />
    {:else}
      <LogTimeline entries={$recentLogs.data} />
    {/if}
  </section>
</div>
