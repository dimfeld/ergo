<script lang="typescript">
  import '../app.css';
  import { page } from '$app/stores';
  import { setContext } from 'svelte';
  import { writable } from 'svelte/store';
  import { createDarkStore, cssDarkModePreference } from '../styles';
  import { createHeaderTextStore } from '^/header';
  import ky from 'ky';
  import Nav from './_Nav.svelte';
  import { QueryClient, QueryClientProvider } from '@sveltestack/svelte-query';

  // Hack in the API key until we support actual user login.
  // @ts-ignore
  const apiClient = window.ERGO_API_KEY
    ? ky.extend({
        headers: {
          // @ts-ignore
          Authorization: 'Bearer ' + window.ERGO_API_KEY,
        },
      })
    : ky;

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

  let headerText = createHeaderTextStore();
</script>

<QueryClientProvider client={queryClient}>
  <div
    id="top"
    class:dark={darkMode}
    class="min-h-screen overflow-y-auto overflow-x-hidden flex flex-col"
  >
    <Nav {section} />
    <header class="bg-white dark:bg-black shadow-sm">
      <div class="mx-auto py-4 px-4 sm:px-6 lg:px-8">
        <h1 class="text-lg leading-6 font-semibold text-gray-900 dark:text-gray-100">
          {$headerText}
        </h1>
      </div>
    </header>
    <main class="flex-grow w-full mx-auto py-10 px-4 sm:px-6 lg:px-8">
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
