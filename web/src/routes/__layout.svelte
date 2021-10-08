<script context="module" lang="ts">
  import { createApiClient, loadFetch } from '$lib/api';
  import type { Load } from '@sveltejs/kit';

  export const load: Load = async function load({ fetch }) {
    fetch = loadFetch(fetch);
    let [inputs, actions] = await Promise.all([
      fetch('/api/inputs').then((r) => r.json()),
      fetch('/api/actions').then((r) => r.json()),
    ]);

    return {
      props: {
        inputs,
        actions,
      },
    };
  };
</script>

<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  import { createDarkStore, cssDarkModePreference } from '../styles';
  import { createHeaderTextStore } from '$lib/header';
  import { setApiClientContext } from '$lib/api';
  import Nav from './_Nav.svelte';
  import { QueryClient, QueryClientProvider } from '@sveltestack/svelte-query';
  import { Input, Action } from '../api_types';
  import { setBaseData } from '../data';

  export let inputs: Input[];
  export let actions: Action[];

  setBaseData(inputs, actions);

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
  $: section = $page.path.split('/')[1];

  let headerTextList = createHeaderTextStore();
  $: titleText = $headerTextList.slice().reverse().join(' - ');
</script>

<svelte:head>
  <title>{titleText} - Ergo</title>
</svelte:head>

<QueryClientProvider client={queryClient}>
  <div
    id="top"
    class:dark={darkMode}
    class="min-h-screen overflow-y-auto overflow-x-hidden flex flex-col"
  >
    <Nav {section} />
    <header class="bg-white dark:bg-black shadow-sm">
      <div class="mx-auto py-4 px-4 sm:px-6 lg:px-8">
        <h1
          class="text-lg leading-6 font-semibold text-gray-900 dark:text-gray-100 flex space-x-2 items-center"
        >
          {#each $headerTextList as t, i}
            {#if i > 0}
              <svg
                class="flex-shrink-0 h-5 w-5 text-gray-400 dark:text-gray-600"
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
    <main class="flex-grow w-full mx-auto py-10 px-4 sm:px-6 lg:px-8 flex flex-col">
      <slot />
    </main>
  </div>
</QueryClientProvider>

<style lang="postcss">
  #top {
    @apply bg-gray-50 text-gray-900;
  }

  #top.dark {
    @apply bg-gray-900 text-gray-100;
  }
</style>
