<script lang="ts">
  export let pointer = true;
  export let wrap = false;

  let classNames = '';
  export { classNames as class };
</script>

<div
  class="py-2 px-3 hover:bg-primary-100 dark:hover:bg-primary-800 sm:text-sm text-gray-700 dark:text-gray-300 {classNames}"
  class:whitespace-nowrap={!wrap}
  class:cursor-pointer={pointer}
  on:click
>
  <slot />
</div>
