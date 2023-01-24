<script lang="ts">
  import CanvasTitledBox from '../canvas/CanvasTitledBox.svelte';
  import Editor from '../Editor.svelte';
  import { createEventDispatcher } from 'svelte';
  import type { DataFlowManagerNode, JsFunctionType } from './dataflow_manager';
  import type { SelectionState } from '../canvas/drag';

  const dispatch = createEventDispatcher();

  export let node: DataFlowManagerNode;
  export let selectMode = false;
  export let selected: SelectionState = null;

  $: if (!node.meta.format) {
    node.meta.format = 'expression';
  }
</script>

<CanvasTitledBox
  bind:name={node.config.name}
  bind:position={node.meta.position}
  {selectMode}
  {selected}
  on:mousemove
  on:mouseleave
  on:selectModeClick>
  <div slot="title" class="flex w-full items-center gap-1">
    <div class="no-drag relative h-full">
      <!-- invisible element for sizing, so that the text box is no larger than needed -->
      <span class="invisible px-2 py-0">{node.config.name}</span>
      <input
        type="text"
        autocomplete="off"
        bind:value={node.config.name}
        class="absolute inset-0 border-transparent !bg-transparent px-2 py-0 text-xs font-medium text-accent-800 hover:border-gray-500 " />
    </div>

    <button
      type="button"
      class="no-drag whitespace-nowrap pt-px text-xs text-accent-800"
      on:click={() => (node.meta.autorun = !node.meta.autorun)}>
      Auto {node.meta.autorun ? 'ON' : 'OFF'}
    </button>

    <button
      type="button"
      class="ml-auto mr-1 text-accent-800"
      on:click={() => dispatch('startAddEdge')}>
      &gt;
    </button>
  </div>

  {#if node.config.func.type === 'js'}
    <div class="flex h-full flex-col">
      <Editor
        class="h-1/3"
        contents={node.meta.contents}
        format="js"
        notifyOnChange={true}
        on:change={(e) => (node.meta.contents = e.detail)}
        toolbar={false} />
      <div class="flex-1 border-t border-gray-500 text-sm">Results</div>
    </div>
  {:else if node.config.func.type === 'trigger'}
    <Editor
      class="h-full"
      contents={node.meta.contents}
      format="json5"
      notifyOnChange={true}
      on:change={(e) => (node.meta.contents = e.detail)}
      toolbar={false} />
  {:else}
    <div class="flex h-full flex-col">
      Type {node.config.func.type} not implemented yet
    </div>
  {/if}
</CanvasTitledBox>
