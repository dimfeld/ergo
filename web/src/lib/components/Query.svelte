<script lang="ts">
  import type { UseQueryStoreResult } from '@sveltestack/svelte-query';
  import Loading from './Loading.svelte';
  import { derived } from 'svelte/store';

  type DATA = $$Generic;
  type ERROR = $$Generic;

  interface $$Slots {
    default: { data: DATA };
    loading: {};
    error: { error: ERROR };
  }

  export let query: UseQueryStoreResult<DATA, ERROR> | undefined;
  export let showLoading = true;
  export let showError = true;
  /** Show the loader after waiting for this long to load */
  export let loaderDelay = 500;

  export let description = '';

  const delayedLoading = derived(
    query,
    (q, set) => void setTimeout(() => set(q?.isLoading ?? false), loaderDelay),
    false
  );

  function handleError(_e: ERROR) {
    // TODO Handle 403, 404, etc. differently
    if (description) {
      return `Failed to load ${description}`;
    }
    return 'Failed to load';
  }
</script>

{#if !query || $query.isSuccess}
  <slot data={$query?.data} />
{:else if $query.isLoading}
  {#if showLoading && $delayedLoading}
    <slot name="loading"><Loading /></slot>
  {/if}
{:else if $query.isError}
  {#if showError}
    <slot name="error" error={$query.error}>{handleError($query.error)}</slot>
  {/if}
{/if}
