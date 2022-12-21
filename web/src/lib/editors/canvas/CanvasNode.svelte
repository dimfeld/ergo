<script lang="ts">
  import { spring } from 'svelte/motion';

  export let position = { x: 0, y: 0 };

  let dragging = false;

  const displayPosition = spring(position, { stiffness: 0.3, damping: 0.5 });
  $: displayPosition.set(position);
  $: roundedPosition = { x: Math.round($displayPosition.x), y: Math.round($displayPosition.y) };

  $: transform = dragging
    ? `translate3d(${roundedPosition.x}px, ${roundedPosition.y}px, 0) scale(1)`
    : `translate(${roundedPosition.x}px, ${roundedPosition.y}px) scale(1)`;
</script>

<div style:transform>
  <slot />
</div>
