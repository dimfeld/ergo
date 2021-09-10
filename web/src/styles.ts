import { getContext, setContext } from 'svelte';
import { writable, Writable } from 'svelte/store';

export function createDarkStore() {
  let initialDarkMode: boolean | null = null;

  if (typeof window === 'undefined') {
    return writable(false);
  }

  if ('theme' in window.localStorage) {
    initialDarkMode = window.localStorage.theme === 'true';
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

export function cssDarkModePreference() {
  if (typeof window === 'undefined') {
    return false;
  }

  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}
