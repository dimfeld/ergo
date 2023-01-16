<script lang="ts">
  import { drag, type Box, type DragPosition, type DragUpdate, type Point } from './drag';
  import { createEventDispatcher } from 'svelte';
  import { cls } from '$lib/styles';

  let className: string | undefined;
  export { className as class };

  const dispatch = createEventDispatcher<{ done: Box }>();

  interface Pos {
    start: Point;
    end: DragPosition;
  }
  let pos: Pos | null = null;

  function handleDrag(change: DragUpdate) {
    if (change.dragging) {
      if (pos) {
        pos.end = change.position;
      } else {
        pos = {
          start: change.mouseStart,
          end: change.position,
        };
      }
    } else if (pos) {
      dispatch('done', toBox(pos, 'target'));
      pos = null;
    }
  }

  function toBox(pos: Pos, endKey: keyof DragPosition): Box {
    let x1 = pos.start.x;
    let y1 = pos.start.y;
    let w = pos.end[endKey].x;
    let h = pos.end[endKey].y;

    let x2 = x1 + w;
    let y2 = y1 + h;

    return {
      x: Math.min(x1, x2),
      y: Math.min(y1, y2),
      w: Math.abs(w),
      h: Math.abs(h),
    };
  }
</script>

<div
  class="absolute inset-0 z-40 cursor-cell"
  use:drag={{
    onChange: handleDrag,
    enableDrag: true,
    enableWheel: false,
    manageStyle: false,
    position: pos?.end.current || { x: 0, y: 0 },
  }}>
  {#if pos}
    {@const box = toBox(pos, 'current')}
    <div
      class={cls('absolute', className)}
      style:left="{box.x}px"
      style:top="{box.y}px"
      style:width="{box.w}px"
      style:height="{box.h}px" />
  {/if}
</div>
