<script lang="ts">
  import type { DataFlowConfig, TaskConfig } from '$lib/api_types';
  import { dataflowManager, type DataFlowSource } from './dataflow_manager';
  import Button from '$lib/components/Button.svelte';
  import Plus from '$lib/components/icons/Plus.svelte';
  import Canvas from './canvas/Canvas.svelte';
  import DrawRectangle from './canvas/DrawRectangle.svelte';
  import type { Box } from './canvas/drag';
  import CanvasTitledBox from './canvas/CanvasTitledBox.svelte';

  export let source: DataFlowSource;
  export let compiled: DataFlowConfig;

  $: data = dataflowManager(compiled, source);

  export function getState(): { compiled: TaskConfig; source: any } {
    let { compiled, source } = data.compile();

    return {
      compiled: {
        type: 'DataFlow',
        data: compiled,
      },
      source: {
        type: 'DataFlow',
        data: source,
      },
    };
  }

  type EditorState = 'normal' | 'addingNode';
  let state: EditorState = 'normal';
  function toggleState(newState: EditorState) {
    state = newState === state ? 'normal' : newState;
  }

  function addNode(box: Box) {
    data.addNode({
      x: box.x,
      y: box.y,
      w: Math.max(box.w, 150),
      h: Math.max(box.h, 150),
    });
    state = 'normal';
  }

  let addButtonEl: HTMLButtonElement;

  let canvasPosition = { x: 0, y: 0 };
</script>

<svelte:window
  on:keydown={(e) => {
    if (e.key === 'Escape') {
      state = 'normal';
      addButtonEl?.blur();
    }
  }} />

<div class="relative">
  <Canvas bind:position={canvasPosition} scrollable={false}>
    {#each $data.nodes as node, i (node.config.name)}
      <CanvasTitledBox bind:position={node.meta.position} name={node.config.name} />
    {/each}

    <div slot="controls">
      <div class="absolute top-4 left-4 z-50 flex gap-2 overflow-visible">
        <span>{state}</span>
        <Button bind:element={addButtonEl} iconButton on:click={() => toggleState('addingNode')}>
          <Plus />
        </Button>
      </div>

      {#if state === 'addingNode'}
        <DrawRectangle
          on:done={(e) => addNode(e.detail)}
          class="border-2 border-daccent-100 bg-accent-500/25" />
      {/if}
    </div>
  </Canvas>
</div>
