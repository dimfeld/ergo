import { loadFetch } from '$lib/api';
import type { LayoutLoad } from './$types';
import initWasm from '$lib/wasm';
import type {
  Input,
  Action,
  ActionCategory,
  ExecutorInfo,
  AccountType,
  AccountPublicInfo,
} from '$lib/api_types';
import { browser } from '$app/environment';

export const ssr = false;

export const load: LayoutLoad = async function load({ fetch }) {
  if (browser) {
    await initWasm();
  }
  fetch = loadFetch(fetch);
  let [inputList, actionList, actionCategoryList, executorList, accountTypeList, accountList]: [
    Input[],
    Action[],
    ActionCategory[],
    ExecutorInfo[],
    AccountType[],
    AccountPublicInfo[]
  ] = await Promise.all([
    fetch('/api/inputs').then((r) => r.json()),
    fetch('/api/actions').then((r) => r.json()),
    fetch('/api/action_categories').then((r) => r.json()),
    fetch('/api/executors').then((r) => r.json()),
    fetch('/api/account_types').then((r) => r.json()),
    fetch('/api/accounts').then((r) => r.json()),
  ]);

  let inputs = new Map(inputList.map((i) => [i.input_id, i]));
  let actions = new Map(actionList.map((a) => [a.action_id, a]));
  let actionCategories = new Map(actionCategoryList.map((a) => [a.action_category_id, a]));
  let executors = new Map(executorList.map((e) => [e.name, e]));
  let accountTypes = new Map(accountTypeList.map((a) => [a.account_type_id, a]));
  let accounts = new Map(accountList.map((a) => [a.account_id, a]));

  return {
    inputs,
    actions,
    actionCategories,
    accountTypes,
    accounts,
    executors,
  };
};
