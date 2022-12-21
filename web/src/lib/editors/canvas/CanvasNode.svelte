<script lang="ts">
  import { drag, type Point } from './drag';

  export let position = { x: 0, y: 0 };
  export let size = { x: 0, y: 0 };
  export let minSize = { x: 150, y: 150 };

  function enforceMinSize(point: Point) {
    return {
      x: Math.max(point.x, minSize.x),
      y: Math.max(point.y, minSize.y),
    };
  }

  let dragHandleElement: HTMLElement;
</script>

<div
  class="absolute grid gap-2 rounded-xl border-black bg-red-400 px-2 py-2 shadow-xl"
  use:drag={{
    onChange: (c) => (position = c.position.current),
    position,
    dragHandle: dragHandleElement,
  }}
  style:width={size.x + 'px'}
  style:height={size.y + 'px'}>
  <div bind:this={dragHandleElement} class="drag-handle h-4 w-4 cursor-move bg-green-500" />
  <slot />

  <div
    class="resize-handle h-4 w-4 cursor-se-resize place-self-end bg-yellow-500"
    use:drag={{
      onChange: (c) => (size = c.position.current),
      position: size,
      manageStyle: false,
      transformPosition: enforceMinSize,
    }} />
</div>

<style>
  .grid {
    grid-template-columns: 1fr;
    grid-template-rows: auto 1fr auto;
  }
</style>
