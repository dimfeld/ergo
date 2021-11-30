<script lang="ts">
  let classNames: string | undefined = undefined;
  export { classNames as class };

  export let label = '';
</script>

<section
  class="w-full p-4 rounded border border-gray-200 dark:border-gray-400 shadow-md {classNames ||
    ''}"
>
  {#if label}
    <header class="font-bold text-gray-700 dark:text-gray-300 mb-4">{label}</header>
  {/if}
  <slot />
</section>
