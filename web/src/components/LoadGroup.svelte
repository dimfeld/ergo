<script lang="ts">
  import { onDestroy, setContext } from 'svelte';
  import { readable, writable, derived } from 'svelte/store';
  import type { LoadGroupStore, LoadGroupManager } from './loadGroup';
  import { registerLoadGroup } from './loadGroup';

  export let name: string = 'Load Group';
  export let loaderDelay = 0;

  // Since we have to recreate the store that calculates the state to handle changes in the list
  // of tracked stores, this store exists to provide a stable reference for the parent.
  let stateForParent = writable({ isLoading: false, isError: false, error: undefined });
  let unregisterState = registerLoadGroup(name, stateForParent);
  onDestroy(unregisterState);

  let stores = new Map<Symbol, LoadGroupStore>();
  let manager: LoadGroupManager = {
    register(symbol, store) {
      stores.set(symbol, store);
      stores = stores;
    },
    delete(symbol) {
      stores.delete(symbol);
      stores = stores;
    },
  };

  // Make sure to set the context AFTER calling registerLoadGroup, so that we register with the parent (if any)
  // instead of ourselves.
  setContext('loadGroupManager', manager);

  let state: LoadGroupStore;
  $: {
    let storeValues = Array.from(stores.values());
    if (!storeValues.length) {
      state = readable({ isLoading: false, isError: false, error: undefined });
    } else {
      state = derived(
        Array.from(stores.values()) as [LoadGroupStore, ...LoadGroupStore[]],
        (children) => {
          let errorChild = children.find((e) => e.isError);
          if (errorChild) {
            return { isLoading: false, isError: true, error: errorChild.error };
          }

          let loading = children.some((c) => c.isLoading);
          return { isLoading: loading, isError: false, error: undefined };
        }
      );
    }
  }

  $: $stateForParent = $state;
  $: display = $state.isLoading || ($state.isError && $$slots.error) ? 'none' : 'contents';

  let canShowLoader = false;
  $: {
    if ($state.isLoading && !canShowLoader) {
      if (loaderDelay) {
        setTimeout(() => {
          if ($state.isLoading) {
            canShowLoader = true;
          }
        }, loaderDelay);
      } else {
        canShowLoader = true;
      }
    } else if (!$state.isLoading) {
      // Reset so that we'll delay if this goes back to loading state again.
      canShowLoader = false;
    }
  }
</script>

{#if $state.isLoading && canShowLoader}
  <slot name="loading" />
{:else if $state.isError && $$slots.error}
  <slot name="error" error={$state.error} />
{/if}

<div style="display:{display}">
  <slot />
</div>
