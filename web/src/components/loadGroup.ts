import type { Readable } from 'svelte/store';
import { getContext } from 'svelte';

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

export function registerLoadGroup(label: string, store: LoadGroupStore) {
  let sus = getParentLoadGroup();
  if (!sus) {
    // It's ok to have no parent load group. Return a dummy unsubscribe function.
    return () => void 0;
  }

  let symbol = Symbol(label);
  sus.register(symbol, store);
  return () => sus.delete(symbol);
}
