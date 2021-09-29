<script lang="ts">
  import { page } from '$app/stores';
  import type { TaskDescription } from '$lib/api_types';
  import { useQuery } from '@sveltestack/svelte-query';
  import Query from '$lib/components/Query.svelte';
  import TaskRow from '$lib/components/TaskRow.svelte';
  import { getHeaderTextStore } from '$lib/header';
  import sorter from 'sorters';
  getHeaderTextStore().set(['Tasks']);

  const taskQuery = useQuery<TaskDescription[]>('tasks');

  const sortFields = {
    run: { label: 'Recently Run', sort: { value: 'last_triggered', descending: true } },
    name: { label: 'Name', sort: { value: 'name', descending: false } },
    updated: { label: 'Updated', sort: { value: 'modified', descending: true } },
  };

  let sortField = '';
  $: {
    sortField = $page.query.get('sort');
    if (!(sortField in sortFields)) {
      sortField = 'run';
    }
  }
</script>

<Query query={taskQuery} let:data>
  <header class="flex w-full justify-between">
    <p class="flex space-x-4 text-sm">
      <span class="text-gray-700 dark:text-gray-300">Order by</span>
      {#each Object.entries(sortFields) as [key, { label }]}
        <a
          href="?sort={key}"
          class:selected-sort={key === sortField}
          class="sort-field font-medium hover:underline"
          on:click={() => (sortField = key)}>{label}</a
        >
      {/each}
    </p>
    <a href="/tasks/new" class="text-sm">New Task</a>
  </header>
  {#each data.slice().sort(sorter(sortFields[sortField].sort)) as task (task.task_id)}
    <TaskRow {task} />
  {/each}
</Query>

<style lang="postcss">
  .sort-field {
    @apply text-gray-500;

    &.selected-sort {
      @apply text-gray-900 underline;
    }
  }

  :global(.dark) .sort-field {
    @apply text-gray-500;
    &.selected-sort {
      @apply text-gray-100;
    }
  }
</style>
