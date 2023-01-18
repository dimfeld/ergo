<script lang="ts">
  import type { LineEnd, SelectionState } from './drag';
  import { cls } from '$lib/styles';

  export let start: LineEnd;
  export let end: LineEnd;
  export let selectMode = false;
  export let selected: SelectionState = null;
  export let color: string;
  export let dash: string | undefined = undefined;

  $: startPadding = ((start.offset ?? 0) + 1) * 16;
  $: endPadding = ((end.offset ?? 0) + 1) * 16;

  let path: string;

  // Draw a line between the two points, attempting to go around the boxes and making some
  // affordance to avoid collisions.
  $: {
    let xStart = start.point.x + (start.margin ?? 4);
    let xEnd = end.point.x - (end.margin ?? 4);

    let xDelta = end.point.x - start.point.x;
    let yDelta = end.point.y - start.point.y;

    let verticalMain = Math.abs(xDelta) < Math.abs(yDelta);

    let segments = [`M${xStart},${start.point.y} `];

    let currentX = xStart;

    const goAroundStartBox = xStart > xEnd;
    if (!verticalMain || goAroundStartBox) {
      segments.push(`h${startPadding} `);
      currentX += startPadding;
    } else {
      let xMid = (xStart + xEnd) / 2;
      segments.push(`H${xMid} `);
      currentX = xMid;
    }

    // Try not to intersect with the source box. This assumes that the line start is to the right
    // of the source box.
    const approachX = xEnd - endPadding;
    if (xStart > approachX) {
      // First go to the top or bottom of the box, then to the left.
      const destY =
        yDelta > 0 ? start.box.y + start.box.h + startPadding : start.box.y - startPadding;
      currentX = approachX;
      segments.push(`V${destY} H${currentX} `);
    }

    // Try not to intersect with the destination box. This assumes that the line end is to the left
    // of the destination box.
    if (currentX > approachX) {
      // First go to the top or bottom of the box, then to the left.
      const destY = yDelta < 0 ? end.box.y + end.box.h + endPadding : end.box.y - endPadding;
      currentX = xEnd - endPadding;
      segments.push(`V${destY} H${currentX} `);
    }

    segments.push(`V${end.point.y} H${xEnd}`);

    path = segments.join('');
  }

  $: pathClass = cls(
    !selectMode && 'pointer-events-none',
    selected === 'valid' && 'stroke-[4] stroke-accent-500',
    selected === 'invalid' && 'stroke-[4] stroke-red-500'
  );
</script>

<svg
  stroke-width="2"
  width="1"
  height="1"
  fill="none"
  stroke={color}
  stroke-dasharray={dash}
  class="absolute -z-10 overflow-visible">
  <path d={path} stroke-linejoin="bevel" class={pathClass} />
  {#if selectMode}
    <path
      d={path}
      stroke-linejoin="bevel"
      stroke-width="4"
      class="stroke-transparent"
      on:mousemove
      on:mouseleave
      on:click />
  {/if}
</svg>
