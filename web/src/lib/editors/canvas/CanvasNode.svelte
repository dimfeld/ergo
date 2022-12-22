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

  let dragging = false;
  let dragHandleElement: HTMLElement;
</script>

<div
  class="absolute grid gap-2 overflow-hidden rounded-xl border-black bg-dgray-100 shadow-xl"
  use:drag={{
    onChange: (c) => {
      position = c.position.current;
      dragging = c.dragging;
    },
    position,
    dragCursor: 'grabbing',
    dragHandle: dragHandleElement,
  }}
  style:width={size.x + 'px'}
  style:height={size.y + 'px'}>
  <div
    bind:this={dragHandleElement}
    class="drag-handle h-1 w-full bg-dgray-300 drop-shadow"
    class:cursor-grab={!dragging}
    class:cursor-grabbing={dragging} />

  <slot />

  <div
    class="resize-handle h-4 w-4 cursor-nwse-resize place-self-end"
    use:drag={{
      onChange: (c) => (size = c.position.current),
      position: size,
      manageStyle: false,
      transformPosition: enforceMinSize,
      dragCursor: 'nwse-resize',
    }} />
</div>

<style>
  .grid {
    grid-template-columns: 1fr;
    grid-template-rows: auto 1fr auto;
  }
</style>
