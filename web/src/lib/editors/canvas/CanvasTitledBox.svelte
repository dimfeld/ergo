<script lang="ts">
  import { cls } from '$lib/styles';
  import CanvasNode from './CanvasNode.svelte';
  import type { Box, Point } from './drag';
  import DragHandle from './DragHandle.svelte';

  export let name: string;
  export let position: Box;
  export let minSize: Point = { x: 150, y: 150 };
  export let dragDeadZone = 0;
</script>

<CanvasNode
  class="grid grid-cols-1 grid-rows-[auto_1fr] overflow-hidden rounded-lg border bg-gray-100 shadow-xl dark:border-accent-500/25 dark:bg-gray-800"
  bind:position
  {minSize}
  {dragDeadZone}
  let:dragging>
  <DragHandle
    class={cls(
      'flex h-6 w-full items-center bg-accent-300',
      dragging ? 'cursor-grabbing' : 'cursor-grab'
    )}>
    <span class="truncate px-2 text-sm font-medium text-accent-800">{name || ''}</span>
  </DragHandle>
  <div class="p-1">
    <slot />
  </div>
</CanvasNode>
