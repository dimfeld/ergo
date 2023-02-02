<script lang="ts">
  import Drawer from '$lib/components/Drawer.svelte';
  import CanvasTitledBox from '../canvas/CanvasTitledBox.svelte';
  import Editor from '../Editor.svelte';
  import { createEventDispatcher } from 'svelte';
  import type { DataFlowManagerNode, JsFunctionType } from './dataflow_manager';
  import type { SelectionState } from '../canvas/drag';
  import type { NodeError } from './sandbox/messages';
  import capitalize from 'just-capitalize';
  import TextField from '$lib/components/TextField.svelte';
  import Labelled from '$lib/components/Labelled.svelte';

  const dispatch = createEventDispatcher();

  export let node: DataFlowManagerNode;
  export let error: NodeError | undefined;
  export let state: unknown;
  export let selectMode = false;
  export let selected: SelectionState = null;

  $: canTriggerAction = node.config.func.type === 'js' || node.config.func.type === 'action';

  $: if (!node.meta.format) {
    node.meta.format = 'expression';
  }

  let drawerOpen = false;
</script>

<CanvasTitledBox
  bind:name={node.config.name}
  bind:position={node.meta.position}
  {selectMode}
  {selected}
  minSize={{ x: 250, y: 150 }}
  on:mousemove
  on:mouseleave
  on:selectModeClick>
  <div slot="title" class="flex w-full items-center gap-1">
    <button
      type="button"
      class="ml-1 text-accent-800 hover:text-accent-700"
      on:click={() => (drawerOpen = true)}>
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="currentColor"
        class="h-4 w-4">
        <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1" fill="currentColor" />
      </svg>
    </button>

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

    <div class="flex gap-2 pt-px text-xs text-accent-800">
      <button
        type="button"
        class="no-drag whitespace-nowrap hover:text-accent-700"
        on:click={() => (node.meta.autorun = !node.meta.autorun)}>
        Auto {node.meta.autorun ? 'ON' : 'OFF'}
      </button>

      <button
        type="button"
        class="no-drag whitespace-nowrap py-1 hover:text-accent-700"
        on:click={() => dispatch('forceRun')}>
        <!-- heroicons play button -->
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 20 20"
          fill="currentColor"
          class="h-4 w-4">
          <path
            fill-rule="evenodd"
            d="M2 10a8 8 0 1116 0 8 8 0 01-16 0zm6.39-2.908a.75.75 0 01.766.027l3.5 2.25a.75.75 0 010 1.262l-3.5 2.25A.75.75 0 018 12.25v-4.5a.75.75 0 01.39-.658z"
            clip-rule="evenodd" />
        </svg>
      </button>
    </div>

    <!-- Drag handle -->
    <div class="mx-2 h-[7px] w-8 flex-grow self-center border-y-2 border-accent-700/70" />

    <button
      type="button"
      class="ml-auto mr-1 text-accent-800"
      on:click={() => dispatch('startAddEdge')}>
      &gt;
    </button>
  </div>

  {#if node.config.func.type === 'js' || node.config.func.type === 'action'}
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

<Drawer bind:open={drawerOpen} class="p-2">
  <Labelled label="Name" class="w-full">
    <input
      type="text"
      class="w-full"
      value={node.config.name}
      on:input={(e) => dispatch('updateName', e.target.value)} />
  </Labelled>
</Drawer>
