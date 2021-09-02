<script lang="ts">
  import type { UseQueryStoreResult } from '@sveltestack/svelte-query';
  import Loading from './Loading.svelte';
  import { derived } from 'svelte/store';

  export let query: UseQueryStoreResult;
  export let showLoading = true;
  export let showError = true;
  /** Show the loader after waiting for this long to load */
  export let loaderDelay = 500;

  export let description = '';

  const delayedLoading = derived(
    query,
    (q, set) => void setTimeout(() => set(q.isLoading), loaderDelay),
    false
  );

  function handleError(_e: unknown) {
    // TODO Handle 403, 404, etc. differently
    if (description) {
      return `Failed to load ${description}`;
    }
    return 'Failed to load';
  }
</script>

{#if $query.isLoading}
  {#if showLoading && $delayedLoading}
    <slot name="loading"><Loading /></slot>
  {/if}
{:else if $query.isError}
  {#if showError}
    <slot name="error" error={$query.error}>{handleError($query.error)}</slot>
  {/if}
{:else if $query.isSuccess}
  <slot data={$query.data} />
{/if}
