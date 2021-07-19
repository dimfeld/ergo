<script lang="ts">
  import type { Writable } from 'svelte/store';
  import type { InputsLogEntry, TaskDescription } from '../api_types';
  import LogTimeline from '^/components/LogTimeline.svelte';
  import Loading from '^/components/Loading.svelte';
  import TaskRow from '^/components/TaskRow.svelte';
  import { useQuery } from '@sveltestack/svelte-query';
  import { getContext } from 'svelte';
  getContext<Writable<string>>('headerText').set('Dashboard');

  const recentLogs = useQuery<InputsLogEntry[]>('logs');
  const tasks = useQuery<TaskDescription[]>('tasks');
</script>

<div class="flex">
  <section class="flex-grow min-w-max">
    {#if $tasks.isLoading}
      <Loading />
    {:else}
      {#each $tasks.data ?? [] as task (task.id)}
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
