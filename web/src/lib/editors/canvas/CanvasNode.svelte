<script lang="ts">
  import { drag, type Box, type Point } from './drag';

  export let position: Box = { x: 0, y: 0, h: 150, w: 150 };
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
  class="absolute grid overflow-hidden rounded-lg border-black bg-gray-100 shadow-xl dark:bg-gray-800"
  use:drag={{
    onChange: (c) => {
      position.x = c.position.current.x;
      position.y = c.position.current.y;
      dragging = c.dragging;
    },
    position: { x: position.x, y: position.y },
    dragCursor: 'grabbing',
    dragHandle: dragHandleElement,
  }}
  style:width={position.w + 'px'}
  style:height={position.h + 'px'}>
  <div
    bind:this={dragHandleElement}
    class="h-2 w-full bg-accent-300"
    class:cursor-grab={!dragging}
    class:cursor-grabbing={dragging} />

  <div class="p-1">
    <slot />
  </div>

  <div
    class="absolute right-0 bottom-0 h-2 w-2 cursor-nwse-resize place-self-end pt-px"
    use:drag={{
      onChange: (c) => {
        position.w = c.position.current.x;
        position.h = c.position.current.y;
      },
      position: { x: position.w, y: position.h },
      manageStyle: false,
      transformPosition: enforceMinSize,
      dragCursor: 'nwse-resize',
    }} />
</div>

<style>
  .grid {
    grid-template-columns: 1fr;
    grid-template-rows: auto 1fr;
  }
</style>
