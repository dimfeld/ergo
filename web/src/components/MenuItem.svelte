<script lang="ts">
  export let pointer = true;
  export let wrap = false;
</script>

<div
  class="py-2 px-3 hover:bg-primary-100 dark:hover:bg-primary-800"
  class:whitespace-nowrap={!wrap}
  class:cursor-pointer={pointer}
>
  <slot />
</div>
