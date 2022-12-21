<script lang="ts">
  import { spring } from 'svelte/motion';

  /** If true, allow mouse dragging of the canvas. Otherwise it is only moveable via the scrollbars. */
  export let draggable = false;
  export let position = { x: 0, y: 0 };
  /** Dead zone for mouse dragging, in px */
  export let dragDeadZone = 0;

  const displayPosition = spring(position, { stiffness: 0.3, damping: 0.5 });
  $: displayPosition.set(position);
  $: roundedPosition = { x: Math.round($displayPosition.x), y: Math.round($displayPosition.y) };

  function handleWheel(event: WheelEvent) {
    if (event.ctrlKey) {
      // We don't bother with zooming yet
      return;
    }

    event.preventDefault();

    let multiplier = 1;
    switch (event.deltaMode) {
      case WheelEvent.DOM_DELTA_PIXEL:
        multiplier = 0.2;
        break;
      case WheelEvent.DOM_DELTA_LINE:
        multiplier = 2;
        break;
      case WheelEvent.DOM_DELTA_PAGE:
        multiplier = 4;
        break;
    }

    position = {
      x: Math.round($displayPosition.x + event.deltaX * multiplier),
      y: Math.round($displayPosition.y + event.deltaY * multiplier),
    };
  }

  let dragging = false;
  let dragStartPos = { x: 0, y: 0 };
  let dragMouseStartPos = { x: 0, y: 0 };
  function handleDragStart(event: MouseEvent) {
    dragging = true;
    document.body.classList.add('select-none');
    dragStartPos = position;
    dragMouseStartPos = { x: event.clientX, y: event.clientY };

    document.addEventListener('mousemove', handleDragMove, { passive: true });
    document.addEventListener('mouseup', handleDragEnd, { passive: true });
  }

  function handleDragMove(event: MouseEvent) {
    if (dragging) {
      let newPosition = {
        x: Math.round(dragStartPos.x + (event.clientX - dragMouseStartPos.x)),
        y: Math.round(dragStartPos.y + (event.clientY - dragMouseStartPos.y)),
      };

      if (dragDeadZone && position.x === dragStartPos.x && position.y === dragStartPos.y) {
        let delta = Math.sqrt(
          (newPosition.x - dragStartPos.x) ** 2 + (newPosition.y - dragStartPos.y) ** 2
        );

        if (delta < dragDeadZone) {
          return;
        }
      }

      position = newPosition;
    }
  }

  function handleDragEnd() {
    dragging = false;
    document.body.classList.remove('select-none');
    document.removeEventListener('mousemove', handleDragMove);
    document.removeEventListener('mouseup', handleDragEnd);
  }

  function mouseEvents(node: HTMLElement) {
    const wheelHandler = (event: WheelEvent) => {
      if (node === event.target) {
        handleWheel(event);
      }
    };

    const dragHandler = (event: MouseEvent) => {
      if (node === event.target) {
        handleDragStart(event);
      }
    };

    node.addEventListener('wheel', wheelHandler);
    if (draggable) {
      node.addEventListener('mousedown', dragHandler, { passive: true });
    }
    return {
      destroy() {
        node.removeEventListener('wheel', wheelHandler);
        if (draggable) {
          node.removeEventListener('mousedown', dragHandler);
        }
      },
    };
  }
</script>

<div class="absolute inset-0 grid overflow-hidden bg-gray-50" class:cursor-move={dragging}>
  <div class="h-full w-full" use:mouseEvents>
    <div
      class="node-container h-px w-px"
      style:transform="translate({roundedPosition.x}px, {roundedPosition.y}px) scale(1)">
      <slot position={roundedPosition} />
    </div>
  </div>
  {#if $$slots.controls}
    <div class="node-container h-full w-full">
      <slot name="controls" />
    </div>
  {/if}
</div>

<style>
  .grid > * {
    grid-column: 1;
    grid-row: 1;
  }

  /* Disable pointer events on nodes that are just containers, since they should pass through to the canvas. */
  .node-container {
    pointer-events: none;
  }

  /* But we still want events on the actual content, so they can be interactive.
   * Using :where to reduce specificity, so that the containers themselves can override this. */
  :where(.node-container) > :global(:where(*)) {
    pointer-events: auto;
  }
</style>
