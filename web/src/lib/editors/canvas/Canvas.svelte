<script lang="ts">
  import { spring } from 'svelte/motion';

  /** If true, allow mouse dragging of the canvas. Otherwise it is only moveable via the scrollbars. */
  export let draggable = false;

  const position = spring({ x: 0, y: 0 }, { stiffness: 0.3, damping: 0.5 });

  function handleWheel(event: WheelEvent) {
    console.log('handled');
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
    let x = Math.round($position.x + event.deltaX * multiplier);
    let y = Math.round($position.y + event.deltaY * multiplier);

    position.set({ x, y });
  }

  function wheelScrolling(node: HTMLElement) {
    const handler = (event: WheelEvent) => {
      console.log(event);
      if (event.target === node) {
        handleWheel(event);
      }
    };

    node.addEventListener('wheel', handler, { passive: true });
    return {
      destroy() {
        console.log('removing');
        node.removeEventListener('wheel', handler);
      },
    };
  }

  $: pos = { x: Math.round($position.x), y: Math.round($position.y) };
</script>

<svelte:body on:wheel={(e) => console.log('global', e)} />

<div class="absolute inset-0 grid overflow-hidden bg-gray-50">
  <div
    class="h-full w-full"
    style:transform="translate({pos.x}px, {pos.y}px) scale(1)"
    use:wheelScrolling>
    <slot position={pos} />
  </div>
  {#if $$slots.controls}
    <div class="h-full w-full">
      <slot name="controls" />
    </div>
  {/if}
</div>

<style>
  .grid > * {
    grid-column: 1;
    grid-row: 1;
  }
</style>
