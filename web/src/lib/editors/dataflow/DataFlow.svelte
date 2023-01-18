<script lang="ts">
  import type { DataFlowConfig, DataFlowEdge, TaskConfig } from '$lib/api_types';
  import {
    dataflowManager,
    type DataFlowManagerNode,
    type DataFlowSource,
  } from './dataflow_manager';
  import Button from '$lib/components/Button.svelte';
  import Plus from '$lib/components/icons/Plus.svelte';
  import XIcon from '$lib/components/icons/X.svelte';
  import Canvas from '../canvas/Canvas.svelte';
  import DrawRectangle from '../canvas/DrawRectangle.svelte';
  import type { Box, LineEnd, Point, SelectionState } from '../canvas/drag';
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

  type EditorState = 'normal' | 'addingNode' | 'addingEdge' | 'removing';
  let state: EditorState = 'normal';
  function toggleState(newState: EditorState) {
    if (newState === state) {
      enterNormalState();
    } else {
      state = newState;
    }
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

  function handleSelectModeClickNode(node: DataFlowManagerNode) {
    if (state === 'addingEdge' && edgeSourceNode && canAddEdge === 'valid') {
      data.addEdge(edgeSourceNode.meta.id, node.meta.id);
    } else if (state === 'removing') {
      data.deleteNode(node.meta.id);
    }

    enterNormalState();
  }

  $: checkAddEdge =
    edgeSourceNode && edgeDestNode
      ? $data.checkAddEdge(edgeSourceNode.meta.id, edgeDestNode.meta.id)
      : null;

  let canAddEdge: SelectionState;
  $: canAddEdge = checkAddEdge ? 'invalid' : 'valid';

  $: selectMode = state === 'addingEdge' || state === 'removing';

  let removeHighlightedNode: DataFlowManagerNode | null = null;
  let removeHighlightedEdge: DataFlowEdge | null = null;

  function handleMouseMoveNode(node: DataFlowManagerNode) {
    if (state === 'addingEdge' && node !== edgeSourceNode) {
      edgeDestNode = node;
    }

    if (state === 'removing') {
      removeHighlightedNode = node;
    }
  }

  function handleMouseLeaveNode(node: DataFlowManagerNode) {
    if (state === 'addingEdge' && node === edgeDestNode) {
      edgeDestNode = undefined;
    }

    if (state === 'removing' && removeHighlightedNode === node) {
      removeHighlightedNode = undefined;
    }
  }

  function handleMouseMoveEdge(edge: DataFlowEdge) {
    if (state === 'removing') {
      removeHighlightedEdge = edge;
    }
  }

  function handleMouseLeaveEdge(edge: DataFlowEdge) {
    if (state === 'removing' && removeHighlightedEdge === edge) {
      removeHighlightedEdge = undefined;
    }
  }

  function handleClickEdge(edge: DataFlowEdge) {
    if (state === 'removing') {
      data.deleteEdge(edge.from, edge.to);
      enterNormalState();
    }
  }

  function enterNormalState() {
    state = 'normal';
    addButtonEl?.blur();
    removeButtonEl?.blur();
    edgeSourceNode = null;
    edgeDestNode = null;
    removeHighlightedEdge = null;
    removeHighlightedNode = null;
  }

  let addButtonEl: HTMLButtonElement;
  let removeButtonEl: HTMLButtonElement;

  let canvasPosition = { x: 0, y: 0 };

  $: nodes = $data.nodes.map((node) => {
    let selected: SelectionState = null;
    if (state === 'addingEdge' && node === edgeDestNode) {
      selected = canAddEdge;
    } else if (state === 'removing' && node === removeHighlightedNode) {
      selected = 'valid';
    }

    return {
      node,
      selected,
    };
  });
</script>

<svelte:window
  on:keydown={(e) => {
    if (e.key === 'Escape') {
      enterNormalState();
    }
  }} />

<div class="relative">
  <Canvas bind:position={canvasPosition} scrollable={false}>
    {#each nodes as { node, selected } (node.meta.id)}
      <DataFlowNode
        bind:node
        on:startAddEdge={() => startAddEdge(node)}
        {selectMode}
        {selected}
        on:selectModeClick={() => handleSelectModeClickNode(node)}
        on:mousemove={() => handleMouseMoveNode(node)}
        on:mouseleave={() => handleMouseLeaveNode(node)} />
    {/each}

    {#each $data.edges as edge (`${edge.from}-${edge.to}`)}
      <BoxToBoxArrow
        start={dataFlowEdgeSourcePos($data.nodes[$data.nodeIdToIndex.get(edge.from)])}
        end={dataFlowEdgeDestPos($data.nodes[$data.nodeIdToIndex.get(edge.to)])}
        color="rgb(128, 128, 128)"
        {selectMode}
        selected={state === 'removing' && edge === removeHighlightedEdge ? 'valid' : null}
        on:mousemove={() => handleMouseMoveEdge(edge)}
        on:mouseleave={() => handleMouseLeaveEdge(edge)}
        on:click={() => handleClickEdge(edge)} />
    {/each}

    {#if state === 'addingEdge' && edgeSourceNode && edgeDestNode}
      <BoxToBoxArrow
        start={dataFlowEdgeSourcePos(edgeSourceNode)}
        end={dataFlowEdgeDestPos(edgeDestNode)}
        color={canAddEdge === 'valid' ? 'rgb(128, 128, 128)' : '#a03030'}
        dash="4 4" />
    {/if}

    <div slot="controls">
      <div class="absolute top-4 left-4 z-50 flex gap-2 overflow-visible">
        <Button bind:element={addButtonEl} iconButton on:click={() => toggleState('addingNode')}>
          <Plus />
        </Button>
        <Button bind:element={removeButtonEl} iconButton on:click={() => toggleState('removing')}>
          <XIcon />
        </Button>

        {#if checkAddEdge}
          <span>{checkAddEdge}</span>
        {/if}
      </div>

      {#if state === 'addingNode'}
        <DrawRectangle
          on:done={(e) => addNode(e.detail)}
          class="border-2 border-daccent-100 bg-accent-500/25" />
      {/if}
    </div>
  </Canvas>
</div>
