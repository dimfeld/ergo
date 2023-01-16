<script lang="ts">
  import { setContext } from 'svelte';
  import { cls } from '$lib/styles';
  import { drag, type Box, type Point } from './drag';

  export let position: Box = { x: 0, y: 0, h: 150, w: 150 };
  let className: string | undefined;
  export { className as class };
  export let minSize = { x: 150, y: 150 };
  export let dragDeadZone = 0;

  function enforceMinSize(point: Point) {
    return {
      x: Math.max(point.x, minSize.x),
      y: Math.max(point.y, minSize.y),
    };
  }

  let dragging = false;
  let dragHandleElement: HTMLElement;

  setContext('setDragHandle', (element: HTMLElement) => {
    dragHandleElement = element;
  });
</script>

<div
  class={cls('absolute', className)}
  use:drag={{
    onChange: (c) => {
      position.x = c.position.current.x;
      position.y = c.position.current.y;
      dragging = c.dragging;
    },
    position: { x: position.x, y: position.y },
    dragCursor: 'grabbing',
    dragHandle: dragHandleElement,
    deadZone: dragDeadZone,
  }}
  style:width={position.w + 'px'}
  style:height={position.h + 'px'}>
  <slot {dragging} />

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
