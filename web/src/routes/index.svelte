<script lang="ts">
  import type { InputsLogEntry } from '../api_types';
  import LogTimeline from '^/components/LogTimeline.svelte';
  import { useQuery } from '@sveltestack/svelte-query';
  import { getContext } from 'svelte';
  getContext('headerText').set('Dashboard');

  const recentLogs = useQuery<InputsLogEntry[]>('logs');
</script>

<main class="flex">
  <div>Task list here</div>
  <LogTimeline
    class="w-1/4 border-l border-gray-200 dark:border-gray-700"
    entries={$recentLogs.data}
  />
</main>
