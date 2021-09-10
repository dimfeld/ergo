import { Readable } from 'svelte/store';
import { getContext, onDestroy } from 'svelte';

export interface LoadGroupStoreData {
  isLoading: boolean;
  isError: boolean;
  error: Error | undefined;
}

/** A store with the necessary data for a load group manager to process. This, not coincidentally,
 * overlaps with the interface for a svelte-query Query store */
export type LoadGroupStore = Readable<LoadGroupStoreData>;

export type UnregisterFunc = () => void;

export interface LoadGroupManager {
  register(symbol: symbol, store: LoadGroupStore): void;
  delete(symbol: symbol): void;
}

export function getParentLoadGroup() {
  return getContext<LoadGroupManager | undefined>('loadGroupManager');
}

export function registerWithParent(group: LoadGroupManager, store: LoadGroupStore, label?: string) {
  if (!group) {
    // It's ok to have no parent load group. Return a dummy unsubscribe function.
    return () => void 0;
  }

  let symbol = Symbol(label);
  group.register(symbol, store);
  return () => group.delete(symbol);
}

export function registerLoadGroup(store: LoadGroupStore, label?: string) {
  let group = getParentLoadGroup();
  return registerWithParent(group, store, label);
}

/** Register the load group and automatically unregister it when the component is destroyed.
 * Returns the `store` argument for convenience. */
export function autoregisterLoadGroup<
  DATA extends LoadGroupStoreData,
  STORE extends Readable<DATA>
>(store: STORE, label?: string): STORE {
  let destroy = registerLoadGroup(store, label);
  onDestroy(destroy);
  return store;
}
