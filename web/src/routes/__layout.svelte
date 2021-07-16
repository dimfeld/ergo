<script lang="typescript">
  import '../app.css';
  import { page } from '$app/stores';
  import { setContext } from 'svelte';
  import { writable } from 'svelte/store';
  import { createDarkStore, cssDarkModePreference } from '../styles';
  import ky from 'ky';
  import Nav from './_Nav.svelte';
  import { QueryClient, QueryClientProvider } from '@sveltestack/svelte-query';

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        queryFn: ({ queryKey }) => ky(`/api/${queryKey}`).json(),
        staleTime: 60000,
      },
    },
  });

  let darkModeStore = createDarkStore();
  $: darkMode = $darkModeStore ?? cssDarkModePreference();
  $: section = $page.path.split('/')[1];

  let headerTextStore = writable('');
  setContext('headerText', headerTextStore);
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
          {$headerTextStore}
        </h1>
      </div>
    </header>
    <main class="flex-grow">
      <div class="mx-auto py-6 sm:px-6 lg:px-8">
        <div class="px-4 py-4 sm:px-0">
          <slot />
        </div>
      </div>
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
