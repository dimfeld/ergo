<script context="module" lang="ts">
  import { createApiClient, loadFetch } from '$lib/api';
  import type { Load } from '@sveltejs/kit';
  import initWasm from '$lib/wasm';
  import { browser } from '$app/env';

  export const load: Load = async function load({ fetch }) {
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
      props: {
        inputs,
        actions,
        actionCategories,
        accountTypes,
        accounts,
        executors,
      },
      stuff: {
        inputs,
        actions,
        actionCategories,
        executors,
      },
    };
  };
</script>

<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  import { createDarkStore, cssDarkModePreference } from '$lib/styles';
  import { createHeaderTextStore } from '$lib/header';
  import { setApiClientContext } from '$lib/api';
  import Nav from './_Nav.svelte';
  import { QueryClient, QueryClientProvider } from '@sveltestack/svelte-query';
  import {
    Input,
    Action,
    ActionCategory,
    ExecutorInfo,
    AccountType,
    AccountPublicInfo,
  } from '$lib/api_types';
  import { initBaseData } from '$lib/data';

  export let inputs: Map<string, Input>;
  export let actions: Map<string, Action>;
  export let actionCategories: Map<string, ActionCategory>;
  export let executors: Map<string, ExecutorInfo>;
  export let accountTypes: Map<string, AccountType>;
  export let accounts: Map<string, AccountPublicInfo>;

  const {
    inputs: inputStore,
    actions: actionStore,
    actionCategories: actionCategoryStore,
    executors: executorStore,
    accountTypes: accountTypesStore,
    accounts: accountStore,
  } = initBaseData();

  $: $inputStore = inputs;
  $: $actionStore = actions;
  $: $actionCategoryStore = actionCategories;
  $: $executorStore = executors;
  $: $accountTypesStore = accountTypes;
  $: $accountStore = accounts;

  const apiClient = createApiClient();
  setApiClientContext(apiClient);
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        queryFn: ({ queryKey }) => {
          let path = Array.isArray(queryKey) ? queryKey.join('/') : queryKey;
          return apiClient(`/api/${path}`).json();
        },
        staleTime: 60000,
      },
    },
  });

  let darkModeStore = createDarkStore();
  $: darkMode = $darkModeStore ?? cssDarkModePreference();
  $: section = $page.url.pathname.split('/')[1];

  let headerTextList = createHeaderTextStore();
  $: titleText = $headerTextList.slice().reverse().join(' - ');

  $: if (typeof document !== 'undefined') {
    if (darkMode) {
      document.body.classList.add('dark');
    } else {
      document.body.classList.remove('dark');
    }
  }
</script>

<svelte:head>
  <title>{titleText} - Ergo</title>
</svelte:head>

<QueryClientProvider client={queryClient}>
  <div id="top" class="flex h-screen flex-col overflow-y-auto overflow-x-hidden">
    <Nav {section} />
    <header class="bg-white shadow-sm dark:bg-black">
      <div class="mx-auto py-4 px-4 sm:px-6 lg:px-8">
        <h1
          class="flex items-center space-x-2 text-lg font-semibold leading-6 text-gray-900 dark:text-gray-100"
        >
          {#each $headerTextList as t, i}
            {#if i > 0}
              <svg
                class="h-5 w-5 flex-shrink-0 text-gray-400 dark:text-gray-600"
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 20 20"
                fill="currentColor"
                aria-hidden="true"
              >
                <path
                  fill-rule="evenodd"
                  d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
                  clip-rule="evenodd"
                />
              </svg>
            {/if}
            <span class="text-gray-700 dark:text-gray-300">{t}</span>
          {/each}
        </h1>
      </div>
    </header>
    <main class="mx-auto flex w-full flex-grow flex-col py-10 px-4 sm:px-6 lg:px-8">
      <slot />
    </main>
  </div>
</QueryClientProvider>
