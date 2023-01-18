<script lang="ts">
  import type { DataFlowConfig, TaskConfig } from '$lib/api_types';
  import {
    dataflowManager,
    type DataFlowManagerNode,
    type DataFlowSource,
  } from './dataflow_manager';
  import Button from '$lib/components/Button.svelte';
  import Plus from '$lib/components/icons/Plus.svelte';
  import Canvas from '../canvas/Canvas.svelte';
  import DrawRectangle from '../canvas/DrawRectangle.svelte';
  import type { Box, LineEnd, Point } from '../canvas/drag';
  import CanvasTitledBox from '../canvas/CanvasTitledBox.svelte';
  import DataFlowNode from './DataFlowNode.svelte';
  import BoxToBoxArrow from '../canvas/BoxToBoxArrow.svelte';

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

  type EditorState = 'normal' | 'addingNode' | 'addingEdge';
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

  let edgeSourceNode: DataFlowManagerNode | null = null;
  let edgeDestNode: DataFlowManagerNode | null = null;
  function startAddEdge(sourceNode: DataFlowManagerNode) {
    state = 'addingEdge';
    edgeSourceNode = sourceNode;
  }

  function dataFlowEdgeSourcePos(node: DataFlowManagerNode): LineEnd {
    let { position } = node.meta;
    return {
      box: node.meta.position,
      point: {
        x: position.x + position.w,
        y: position.y + 20,
      },
    };
  }

  function dataFlowEdgeDestPos(node: DataFlowManagerNode): LineEnd {
    let { position } = node.meta;
    return {
      box: node.meta.position,
      point: {
        x: position.x,
        y: position.y + 20,
      },
    };
  }

  function handleAddEdge(destNode: DataFlowManagerNode) {
    if (edgeSourceNode) {
      data.addEdge(edgeSourceNode.meta.id, destNode.meta.id);
    }
    state = 'normal';
    edgeSourceNode = null;
    edgeDestNode = null;
  }

  let addButtonEl: HTMLButtonElement;

  let canvasPosition = { x: 0, y: 0 };
</script>

<svelte:window
  on:keydown={(e) => {
    if (e.key === 'Escape') {
      state = 'normal';
      addButtonEl?.blur();
      edgeSourceNode = null;
      edgeDestNode = null;
    }
  }} />

<div class="relative">
  <Canvas bind:position={canvasPosition} scrollable={false}>
    {#each $data.nodes as node (node.meta.id)}
      <DataFlowNode
        bind:node
        on:startAddEdge={() => startAddEdge(node)}
        selectMode={state === 'addingEdge'}
        selected={node === edgeDestNode}
        on:selectModeClick={() => handleAddEdge(node)}
        on:mousemove={() => {
          if (state === 'addingEdge' && node !== edgeSourceNode) {
            edgeDestNode = node;
          }
        }}
        on:mouseleave={() => {
          if (state === 'addingEdge' && node === edgeDestNode) {
            edgeDestNode = undefined;
          }
        }} />
    {/each}

    {#each $data.edges as edge (`${edge.from}-${edge.to}`)}
      <BoxToBoxArrow
        start={dataFlowEdgeSourcePos($data.nodes[$data.nodeIdToIndex.get(edge.from)])}
        end={dataFlowEdgeDestPos($data.nodes[$data.nodeIdToIndex.get(edge.to)])}
        color="rgb(128, 128, 128)" />
    {/each}

    {#if state === 'addingEdge' && edgeSourceNode && edgeDestNode}
      <BoxToBoxArrow
        start={dataFlowEdgeSourcePos(edgeSourceNode)}
        end={dataFlowEdgeDestPos(edgeDestNode)}
        color="rgb(128, 128, 128)" />
    {/if}

    <div slot="controls">
      <div class="absolute top-4 left-4 z-50 flex gap-2 overflow-visible">
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
