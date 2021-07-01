import { getContext, setContext } from 'svelte';
import { Writable, writable } from 'svelte/store';

export function createDarkStore() {
  let initialDarkMode: boolean | null = null;
  if ('theme' in localStorage) {
    initialDarkMode = localStorage.theme;
  }

  let darkModeStore = writable(initialDarkMode);

  let s = {
    ...darkModeStore,
    set(value: boolean | null) {
      localStorage.theme = value;
      darkModeStore.set(value);
    },
  };

  setContext('darkModeStore', s);
  return s;
}

export function darkModeStore() {
  return getContext<Writable<boolean | null>>('darkModeStore');
}
