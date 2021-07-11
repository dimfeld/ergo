<script lang="ts">
  export let element: HTMLButtonElement | undefined = undefined;
  export let disabled = false;

  let classNames = '';
  export { classNames as class };
</script>

<button
  bind:this={element}
  type="button"
  {disabled}
  on:click
  class="inline-flex justify-center rounded-md border border-accent-300 dark:border-accent-700 shadow-sm px-4 py-2 bg-white dark:bg-black text-sm font-medium text-gray-700 disabled:text-gray-400 dark:text-gray-300 dark:disabled:text-gray-400 hover:bg-accent-50 dark:hover:bg-gray-900 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-100 dark:focus:ring-offset-gray-900 focus:ring-accent-500 dark:focus:ring-accent-700 {classNames}"
  class:cursor-not-allowed={disabled}
  ><slot />
</button>
