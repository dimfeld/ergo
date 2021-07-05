<script lang="typescript">
  import '../app.css';
  import { page } from '$app/stores';
  import { createDarkStore, cssDarkModePreference } from '../styles';
  import Nav from './_Nav.svelte';

  let darkModeStore = createDarkStore();
  $: darkMode = $darkModeStore ?? cssDarkModePreference();
  $: section = $page.path.split('/')[1];
</script>

<div id="top" class:dark={darkMode} class="min-h-screen">
  <Nav {section} />
  <header class="bg-white dark:bg-black shadow-sm">
    <div class="max-w-7xl mx-auto py-4 px-4 sm:px-6 lg:px-8">
      <h1 class="text-lg leading-6 font-semibold text-gray-900 dark:text-gray-100">Dashboard</h1>
    </div>
  </header>
  <main>
    <div class="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
      <div class="px-4 py-4 sm:px-0">
        <slot />
      </div>
    </div>
  </main>
</div>

<style lang="postcss">
  #top {
    @apply bg-gray-50 text-gray-900;
  }

  #top.dark {
    @apply bg-gray-900 text-gray-100;
  }
</style>
