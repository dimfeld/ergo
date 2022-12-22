<script lang="ts">
  import { getBoxToBoxArrow } from 'perfect-arrows';

  interface Box {
    x: number;
    y: number;
    w: number;
    h: number;
  }

  export let from: Box;
  export let to: Box;

  $: [sx, sy, cx, cy, ex, ey, ae, as, ac] = getBoxToBoxArrow(
    from.x,
    from.y,
    from.w,
    from.h,
    to.x,
    to.y,
    to.w,
    to.h,
    {
      bow: 0,
      straights: true,
      padStart: 10,
      padEnd: 20,
    }
  );
  $: endAngleAsDegrees = (ae * 180) / Math.PI;
</script>

<svg
  stroke-width="2"
  width="1"
  height="1"
  class="pointer-events-none absolute overflow-visible fill-accent-300 stroke-accent-300">
  <path d={`M${sx},${sy} Q${cx},${cy} ${ex},${ey}`} fill="none" />
  <polygon
    points="0,-6 12,0, 0,6"
    transform={`translate(${ex},${ey}) rotate(${endAngleAsDegrees})`} />
</svg>
