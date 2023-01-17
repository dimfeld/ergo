<script lang="ts">
  import { dragHandleSetter } from './drag';

  let className: string | undefined;
  export { className as class };

  const setDragHandle = dragHandleSetter();

  let dragHandle: HTMLElement;

  $: setDragHandle(dragHandle);
</script>

<div bind:this={dragHandle} class={className}>
  <slot />
</div>
