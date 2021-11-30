import { Input, Action, ExecutorInfo } from './api_types';
import { getContext, setContext } from 'svelte';
import { writable, Writable } from 'svelte/store';

const KEY = 'ergo_base_data';

export interface BaseData {
  inputs: Writable<Map<string, Input>>;
  actions: Writable<Map<string, Action>>;
  executors: Writable<Map<string, ExecutorInfo>>;
}

export function initBaseData() {
  let stores = {
    inputs: writable(new Map()),
    actions: writable(new Map()),
    executors: writable(new Map()),
  };

  setContext(KEY, stores);
  return stores;
}

export function baseData(): BaseData {
  return getContext(KEY);
}
