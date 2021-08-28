<script lang="ts">
  import { useQuery } from '@sveltestack/svelte-query';
  import { readable } from 'svelte/store';
  import { getStores } from '$app/stores';
  import Loading from '^/components/Loading.svelte';
  import type { TaskResult } from '^/api_types';

  const { page } = getStores();
  $: task = useQuery<TaskResult>(['tasks', $page.params.task_id]);
</script>

{#if $task.isLoading}
  <Loading />
{:else}
  Task {JSON.stringify($task.data)}
{/if}
