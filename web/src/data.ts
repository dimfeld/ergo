import { Input, ActionPayload } from './api_types';
import { getContext, setContext } from 'svelte';
import { writable, Writable } from 'svelte/store';

const KEY = 'ergo_base_data';

export interface BaseData {
  inputs: Writable<Map<string, Input>>;
  actions: Writable<Map<string, ActionPayload>>;
}

export function setBaseData(inputs: Input[], actions: ActionPayload[]) {
  let inputMap = new Map(inputs.map((i) => [i.input_id, i]));
  let actionMap = new Map(actions.map((a) => [a.action_id, a]));

  setContext(KEY, {
    inputs: writable(inputMap),
    actions: writable(actionMap),
  });
}

export function baseData(): BaseData {
  return getContext(KEY);
}
