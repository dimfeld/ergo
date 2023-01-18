<script lang="ts">
  import { cls } from '$lib/styles';
  import CanvasNode from './CanvasNode.svelte';
  import type { Box, Point } from './drag';
  import { createEventDispatcher } from 'svelte';
  import DragHandle from './DragHandle.svelte';

  const dispatch = createEventDispatcher();

  export let name: string;
  let className: string | undefined;
  export { className as class };
  export let position: Box;
  export let minSize: Point = { x: 150, y: 150 };
  export let dragDeadZone = 0;
  export let dragHandleStrict = false;
  export let selectMode = false;
  export let selected = false;

  $: nodeClass = cls(
    `grid grid-cols-1 grid-rows-[auto_1fr] overflow-hidden rounded-lg border bg-gray-100
      shadow-xl dark:border-accent-500/25 dark:bg-gray-800`,
    selected && 'ring-4 ring-accent-500/75',
    className
  );
</script>

<CanvasNode
  class={nodeClass}
  bind:position
  {minSize}
  {dragDeadZone}
  {dragHandleStrict}
  on:mousemove
  on:mouseleave
  let:dragging>
  {#if selectMode}
    <button
      type="button"
      class="absolute inset-0 z-50 bg-gray-800/25"
      on:click={() => dispatch('selectModeClick')} />
  {/if}
  <DragHandle
    class={cls(
      'flex h-6 w-full items-center bg-accent-300',
      dragging ? 'cursor-grabbing' : 'cursor-grab'
    )}>
    <slot name="title">
      <span class="truncate px-2 text-sm font-medium text-accent-800">{name || ''}</span>
    </slot>
  </DragHandle>
  <div class="p-1">
    <slot />
  </div>
</CanvasNode>
