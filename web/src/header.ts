import { Writable } from 'svelte/store';
import { writable } from 'svelte/store';
import { getContext, setContext } from 'svelte';

const HEADER_TEXT = 'ergoHeaderTextStore';

export function createHeaderTextStore(): Writable<string[]> {
  let s = writable([]);
  setContext(HEADER_TEXT, s);
  return s;
}

export function getHeaderTextStore(): Writable<string[]> {
  return getContext(HEADER_TEXT);
}
