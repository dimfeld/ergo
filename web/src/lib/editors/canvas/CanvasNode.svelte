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
  class="absolute grid overflow-hidden rounded-lg border-black bg-gray-100 shadow-xl dark:bg-gray-800"
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
    class="h-2 w-full bg-accent-300"
    class:cursor-grab={!dragging}
    class:cursor-grabbing={dragging} />

  <div class="p-1">
    <slot />
  </div>

  <div
    class="absolute right-0 bottom-0 h-2 w-2 cursor-nwse-resize place-self-end pt-px"
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
    grid-template-rows: auto 1fr;
  }
</style>
