<script lang="ts">
  import type { Writable } from 'svelte/store';
  import type { InputsLogEntry } from '../api_types';
  import LogTimeline from '^/components/LogTimeline.svelte';
  import { useQuery } from '@sveltestack/svelte-query';
  import { getContext } from 'svelte';
  getContext<Writable<string>>('headerText').set('Dashboard');

  const recentLogs = useQuery<InputsLogEntry[]>('logs');
</script>

<div class="flex">
  <div class="flex-grow">Task list here</div>
  <div class="min-h-full pl-4 w-1/4 border-l border-gray-300 dark:border-gray-700">
    {#if $recentLogs.isLoading}
      Loading...
    {:else}
      <LogTimeline entries={$recentLogs.data} />
    {/if}
  </div>
</div>
