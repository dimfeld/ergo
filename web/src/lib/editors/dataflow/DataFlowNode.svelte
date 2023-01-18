<script lang="ts">
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import ChevronDown from '$lib/components/icons/ChevronDown.svelte';
  import CanvasTitledBox from '../canvas/CanvasTitledBox.svelte';
  import type { DataFlowManagerNode } from './dataflow_manager';

  export let node: DataFlowManagerNode;

  export let menuOpen = false;
</script>

<CanvasTitledBox bind:name={node.config.name} bind:position={node.meta.position}>
  <div slot="title" class="flex items-center">
    <div class="no-drag">
      <Dropdown closeOnClickInside={false}>
        <button
          slot="button"
          type="button"
          class="no-drag text-sm font-bold text-black"
          on:click={() => (menuOpen = !menuOpen)}>
          <ChevronDown />
        </button>

        <Checkbox bind:value={node.meta.autorun} class="w-full px-4" label="Autorun" />
      </Dropdown>
    </div>
    <div class="px-1">
      <input
        type="text"
        autocomplete="off"
        bind:value={node.config.name}
        class="no-drag flex-1 border-transparent !bg-transparent px-2 py-0 text-xs font-medium text-accent-800 hover:border-gray-500 " />
    </div>
  </div>
</CanvasTitledBox>
