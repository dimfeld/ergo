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
  <section class="flex-grow min-w-max">Task list here</section>
  <section class="min-h-full ml-4">
    {#if $recentLogs.isLoading}
      Loading...
    {:else}
      <LogTimeline entries={$recentLogs.data} />
    {/if}
  </section>
</div>
