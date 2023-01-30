<script lang="ts">
  import CanvasTitledBox from '../canvas/CanvasTitledBox.svelte';
  import Editor from '../Editor.svelte';
  import { createEventDispatcher } from 'svelte';
  import type { DataFlowManagerNode, JsFunctionType } from './dataflow_manager';
  import type { SelectionState } from '../canvas/drag';
  import type { NodeError } from './sandbox/messages';
  import capitalize from 'just-capitalize';

  const dispatch = createEventDispatcher();

  export let node: DataFlowManagerNode;
  export let error: NodeError | undefined;
  export let state: unknown;
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
        value={node.config.name}
        on:input={(e) => dispatch('updateName', e.target.value)}
        class="absolute inset-0 border-transparent !bg-transparent px-2 py-0 text-xs font-medium text-accent-800 hover:border-gray-500 " />
    </div>

    <div class="flex gap-4 pt-px text-xs text-accent-800">
      <button
        type="button"
        class="no-drag whitespace-nowrap hover:text-accent-600"
        on:click={() => (node.meta.autorun = !node.meta.autorun)}>
        Auto {node.meta.autorun ? 'ON' : 'OFF'}
      </button>

      <button
        type="button"
        class="no-drag whitespace-nowrap hover:text-accent-600"
        on:click={() => dispatch('forceRun')}>
        RUN
      </button>
    </div>

    <button
      type="button"
      class="ml-auto mr-1 text-accent-800"
      on:click={() => dispatch('startAddEdge')}>
      &gt;
    </button>
  </div>

  {#if node.config.func.type === 'js'}
    <div class="flex h-full min-h-0 flex-col">
      <Editor
        class="h-1/3"
        contents={node.meta.contents}
        format="js"
        notifyOnChange={true}
        on:change={(e) => dispatch('updateContent', e.detail)}
        toolbar={false} />
      <div class="min-h-0 flex-1 overflow-auto border-t border-gray-500 text-sm">
        {#if error}
          {capitalize(error.type)} Error
          <br />
          {error.error.message}
        {:else}
          Results:
          <br />
          <!-- TODO better display of results -->
          {JSON.stringify(state ?? null, null, 2)}
        {/if}
      </div>
    </div>
  {:else if node.config.func.type === 'trigger'}
    <div class="flex h-full flex-col">
      <div class="pb-1 text-xs font-medium text-dgray-600">Test Trigger Payload</div>
      <Editor
        class="h-full"
        contents={node.meta.contents}
        format="json5"
        notifyOnChange={true}
        on:change={(e) => dispatch('updateContent', e.detail)}
        toolbar={false} />
    </div>
  {:else}
    <div class="flex h-full flex-col">
      Type {node.config.func.type} not implemented yet
    </div>
  {/if}
</CanvasTitledBox>
