<script lang="ts">
  export let label: string;

  let classNames: string | undefined = undefined;
  export { classNames as class };
</script>

<div class={classNames}>
  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300"
    ><slot name="label">{label}</slot></label
  >
  <div class="mt-1">
    <slot />
  </div>
</div>
