<script lang="ts">
  import { setContext } from 'svelte';
  import { writable } from 'svelte/store';
  import { drag, type DragUpdate } from './drag';

  /** If true, allow mouse dragging of the canvas. */
  export let draggable = true;
  /** If true, allow mouse wheel scrolling of the canvas. */
  export let scrollable = true;
  export let position = { x: 0, y: 0 };
  /** Dead zone for mouse dragging, in px */
  export let dragDeadZone = 0;

  export let dragging = false;
  export let transform: string | undefined = undefined;
  function handleDrag(change: DragUpdate) {
    dragging = change.dragging;
    position = change.position.current;
    transform = change.transform;
  }

  let canvasContext = { position: writable(position) };
  $: canvasContext.position.set(position);

  setContext('canvasContext', canvasContext);
</script>

<div class="absolute inset-0 grid overflow-hidden bg-dgray-50" class:cursor-move={dragging}>
  <div
    class="h-full w-full"
    use:drag={{
      onChange: handleDrag,
      enableDrag: draggable,
      manageStyle: false,
      dragHandleStrict: true,
      enableWheel: scrollable,
      deadZone: dragDeadZone,
      position,
    }}>
    <div class="node-container h-px w-px" style:transform>
      <slot {position} />
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
