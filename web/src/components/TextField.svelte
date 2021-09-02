<script lang="ts">
  let classNames: string = '';
  export { classNames as class };
  export let value: string;
</script>

<input
  {...$$restProps}
  bind:value
  class="dark:bg-gray-800 focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm border-gray-300 rounded-md {classNames}"
/>
