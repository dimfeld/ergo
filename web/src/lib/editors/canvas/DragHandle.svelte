<script lang="ts">
  import { getContext } from 'svelte';

  let className: string | undefined;
  export { className as class };

  const setDragHandle = getContext('setDragHandle');

  let dragHandle: HTMLElement;

  $: setDragHandle(dragHandle);
</script>

<div bind:this={dragHandle} class={className}>
  <slot />
</div>
