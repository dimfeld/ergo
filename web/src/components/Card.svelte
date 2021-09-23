<script lang="ts">
  let classNames: string | undefined = undefined;
  export { classNames as class };
</script>

<section
  class="w-full p-2 rounded border border-gray-200 dark:border-gray-400 shadow-md {classNames ||
    ''}"
>
  <slot />
</section>
