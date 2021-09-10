<script lang="ts">
  import { setContext } from 'svelte';
  import { readable, writable, derived } from 'svelte/store';
  import {
    autoregisterLoadGroup,
    LoadGroupStore,
    LoadGroupManager,
    LoadGroupStoreData,
  } from './loadGroup';

  /** The name to send to the parent load group, if any. This doesn't have to be unique but could be useful for debugging. */
  export let name: string = 'Load Group';
  /** Show the loading spinner after this many milliseconds have passed. Default is 0 ms.
   */
  export let loaderDelay = 0;
  /** If true (the default), only show the loader once. After loading finishes, never show it again
   * even if one of the data sources goes back into the loading or error state. */
  export let once = true;

  /** If true, this load group will register itself with the parent load group, if any.
   * By default it does not register itself, so that load group will not prevent
   * a parent load group from showing its contents while this one finishes loading. */
  export let registerWithParent = false;

  const NOT_LOADING = { isLoading: false, isError: false, error: undefined };

  // Since we have to recreate the store that calculates the state to handle changes in the list
  // of tracked stores, this store exists to provide a stable reference for the parent.
  let stateForParent = writable({ isLoading: false, isError: false, error: undefined });

  if (registerWithParent) {
    autoregisterLoadGroup(stateForParent, name);
  }

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

  let loadingFinishedOnce = false;
  let state: LoadGroupStore;
  $: {
    let storeValues = Array.from(stores.values());
    if (!storeValues.length) {
      state = readable({ isLoading: false, isError: false, error: undefined });
    } else {
      state = derived(
        Array.from(stores.values()) as [LoadGroupStore, ...LoadGroupStore[]],
        (children) => {
          if (once && loadingFinishedOnce && !$state.isLoading) {
            // We already loaded successfully, and `once` is set so never switch back to another state.
            return NOT_LOADING;
          }

          let errorChild = children.find((e) => e.isError);
          if (errorChild) {
            return { isLoading: false, isError: true, error: errorChild.error };
          }

          let loading = children.some((c) => c.isLoading);
          if (!loading) {
            loadingFinishedOnce = true;
          }

          return { isLoading: loading, isError: false, error: undefined };
        }
      );
    }
  }
  $: $stateForParent = $state;

  $: delayedState = derived<LoadGroupStore, LoadGroupStoreData>(
    state,
    (value, set) => {
      if (loaderDelay) {
        setTimeout(() => set(value), loaderDelay);
      } else {
        set(value);
      }
    },
    NOT_LOADING
  );

  // Hide the content if any of these conditions are true:
  // - We have been loading long enough to show the loader,
  // - This is the first time loading
  // - An error occurred.
  //
  // The first two cases are similar, but distinguished so that we don't hide the content if it goes back
  // into loading state for a short time.
  $: display =
    canShowLoader || ($state.isLoading && !loadingFinishedOnce) || ($state.isError && $$slots.error)
      ? 'none'
      : 'contents';
  $: canShowLoader = $state.isLoading && $delayedState.isLoading;
</script>

{#if $state.isLoading && canShowLoader}
  <slot name="loading" />
{:else if $state.isError && $$slots.error}
  <slot name="error" error={$state.error} />
{/if}

<div style="display:{display}">
  <slot />
</div>
