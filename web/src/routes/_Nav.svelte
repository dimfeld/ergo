<script lang="ts">
  import { darkModeStore } from '$lib/styles';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import MenuItem from '$lib/components/MenuItem.svelte';

  export let section: string;

  const sections = [
    { name: 'Dashboard', route: '' },
    { name: 'Tasks', route: 'tasks' },
    { name: 'Inputs', route: 'inputs' },
    { name: 'Actions', route: 'actions' },
  ];

  const profileMenuItems = [
    { name: 'Profile', route: 'profile' },
    { name: 'Settings', route: 'settings' },
  ];

  let mobileMenuOpen = false;

  let darkMode = darkModeStore();
</script>

<nav class="border-b border-gray-200 bg-white dark:border-none dark:bg-gray-800">
  <div class="mx-auto px-4 sm:px-6 lg:px-8">
    <div class="flex h-16 items-center justify-between">
      <div class="flex items-center">
        <div class="flex-shrink-0">
          <img
            class="h-8 w-8"
            src="https://tailwindui.com/img/logos/workflow-mark-indigo-500.svg"
            alt="Workflow" />
        </div>
        <div class="hidden md:block">
          <div class="ml-10 flex items-baseline space-x-4">
            {#each sections as { name, route }}
              <a
                href="/{route}"
                class:selected={route === section}
                class="nav-link rounded-md px-3 py-2 text-sm font-medium">{name}</a>
            {/each}
          </div>
        </div>
      </div>
      <div class="hidden md:block">
        <div class="ml-4 flex items-center md:ml-6">
          <label class="text-sm text-black dark:text-gray-300"
            ><input type="checkbox" bind:checked={$darkMode} /> Test dark
          </label>
          <button
            class="ml-4 rounded-full bg-gray-200 p-1 text-gray-600 hover:text-black focus:outline-none focus:ring-2 focus:ring-black focus:ring-offset-2 focus:ring-offset-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:hover:text-gray-200 dark:focus:ring-gray-200 dark:focus:ring-offset-gray-800">
            <span class="sr-only">View notifications</span>
            <!-- Heroicon name: outline/bell -->
            <svg
              class="h-6 w-6"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
            </svg>
          </button>

          <!-- Profile dropdown -->
          <div class="relative ml-3">
            <Dropdown position="bottom-end">
              <div slot="button">
                <button
                  type="button"
                  class="flex max-w-xs items-center rounded-full bg-gray-200 text-sm text-black focus:outline-none focus:ring-2 focus:ring-black focus:ring-offset-2 focus:ring-offset-gray-200 dark:bg-gray-800 dark:text-white dark:focus:ring-gray-200 dark:focus:ring-offset-gray-800"
                  id="user-menu-button"
                  aria-expanded="false"
                  aria-haspopup="true">
                  <span class="sr-only">Open user menu</span>
                  <img
                    class="h-8 w-8 rounded-full"
                    src="https://images.unsplash.com/photo-1472099645785-5658abf4ff4e?ixlib=rb-1.2.1&ixid=eyJhcHBfaWQiOjEyMDd9&auto=format&fit=facearea&facepad=2&w=256&h=256&q=80"
                    alt="" />
                </button>
              </div>

              <div class="flex w-48 flex-col items-stretch" role="menu" tabindex="-1">
                {#each profileMenuItems as { name, route }}
                  <a
                    href="/{route}"
                    class="w-full"
                    class:bg-gray-100={route === section}
                    class:dark:bg-gray-800={route === section}
                    role="menuitem">
                    <MenuItem>{name}</MenuItem>
                  </a>
                {/each}
              </div>
            </Dropdown>
          </div>
        </div>
      </div>
      <div class="-mr-2 flex md:hidden">
        <!-- Mobile menu button -->
        <button
          type="button"
          class="inline-flex items-center justify-center rounded-md bg-gray-200 p-2 text-gray-600 hover:bg-gray-300 hover:text-black focus:outline-none focus:ring-2 focus:ring-gray-700 focus:ring-offset-2 focus:ring-offset-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-white dark:focus:ring-gray-200 dark:focus:ring-offset-gray-800"
          aria-controls="mobile-menu"
          aria-expanded="false"
          on:click={() => (mobileMenuOpen = !mobileMenuOpen)}>
          <span class="sr-only">Open main menu</span>
          <!--
              Heroicon name: outline/menu

              Menu open: "hidden", Menu closed: "block"
            -->
          <svg
            class:hidden={mobileMenuOpen}
            class:block={!mobileMenuOpen}
            class="block h-6 w-6"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 6h16M4 12h16M4 18h16" />
          </svg>
          <!--
              Heroicon name: outline/x

              Menu open: "block", Menu closed: "hidden"
            -->
          <svg
            class="h-6 w-6"
            class:hidden={!mobileMenuOpen}
            class:block={mobileMenuOpen}
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  </div>

  <!-- Mobile menu, show/hide based on menu state. -->
  <div class="md:hidden" id="mobile-menu">
    <div class="space-y-1 px-2 pt-2 pb-3 sm:px-3">
      {#each sections as { name, route }}
        <a
          href="/{route}"
          class:selected={route === section}
          class="nav-link block rounded-md px-3 py-2 text-base font-medium">{name}</a>
      {/each}
    </div>
    <div class="border-t border-gray-700 pt-4 pb-3">
      <div class="flex items-center px-5">
        <div class="flex-shrink-0">
          <img
            class="h-10 w-10 rounded-full"
            src="https://images.unsplash.com/photo-1472099645785-5658abf4ff4e?ixlib=rb-1.2.1&ixid=eyJhcHBfaWQiOjEyMDd9&auto=format&fit=facearea&facepad=2&w=256&h=256&q=80"
            alt="" />
        </div>
        <div class="ml-3">
          <div class="text-base font-medium text-gray-800 dark:text-white">Tom Cook</div>
          <div class="text-sm font-medium text-gray-600 dark:text-gray-400">tom@example.com</div>
        </div>
        <label class="ml-auto text-sm text-black dark:text-gray-300"
          ><input type="checkbox" bind:checked={$darkMode} /> Test dark toggle</label>
        <button
          class="ml-4 flex-shrink-0 rounded-full bg-gray-100 p-1 text-gray-600 hover:text-gray-500 focus:outline-none focus:ring-2 focus:ring-black focus:ring-offset-2 focus:ring-offset-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:hover:text-white dark:focus:ring-gray-200 dark:focus:ring-offset-gray-800">
          <span class="sr-only">View notifications</span>
          <!-- Heroicon name: outline/bell -->
          <svg
            class="h-6 w-6"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
          </svg>
        </button>
      </div>
      <div class="mt-3 space-y-1 px-2">
        {#each profileMenuItems as { name, route }}
          <a
            href="/{route}"
            class="block rounded-md px-3 py-2 text-base font-medium text-gray-600 hover:bg-gray-200 hover:text-gray-500 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-white"
            >{name}</a>
        {/each}
      </div>
    </div>
  </div>
</nav>

<style lang="postcss">
  .nav-link.selected {
    @apply bg-gray-200 text-gray-800;
  }

  :global(body.dark) .nav-link.selected {
    @apply bg-black text-white;
  }

  .nav-link:not(.selected) {
    @apply text-gray-800 hover:bg-gray-100 hover:text-black;
  }

  :global(body.dark) .nav-link:not(.selected) {
    @apply text-gray-300 hover:bg-gray-700 hover:text-white;
  }
</style>
