<script lang="ts">
  import { InputsLogEntry, TaskDescription } from '../api_types';
  import LogTimeline from '^/components/LogTimeline.svelte';
  import Query from '^/components/Query.svelte';
  import TaskRow from '^/components/TaskRow.svelte';
  import { useQuery } from '@sveltestack/svelte-query';
  import { getHeaderTextStore } from '^/header';
  import sorter from 'sorters';
  getHeaderTextStore().set(['Dashboard']);

  const recentLogs = useQuery<InputsLogEntry[]>('logs');
  const taskQuery = useQuery<TaskDescription[]>('tasks');

  $: tasks = ($taskQuery.data ?? [])
    .slice()
    .sort(sorter({ value: 'last_triggered', descending: true }));
</script>

<div class="flex">
  <section class="flex-grow min-w-max">
    <Query query={taskQuery}>
      {#each tasks as task (task.task_id)}
        <TaskRow {task} />
      {/each}
    </Query>
  </section>
  <section class="min-h-full ml-4">
    <Query query={recentLogs}>
      <LogTimeline entries={$recentLogs.data} />
    </Query>
  </section>
</div>
