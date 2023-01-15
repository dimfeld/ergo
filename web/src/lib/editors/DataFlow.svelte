<script lang="ts">
  import type { DataFlowConfig, TaskConfig } from '$lib/api_types';
  import { dataflowManager, type DataFlowSource } from './dataflow_manager';
  import Button from '$lib/components/Button.svelte';
  import Plus from '$lib/components/icons/Plus.svelte';
  import Canvas from './canvas/Canvas.svelte';

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
</script>

<div class="relative">
  <Canvas scrollable={false}>
    <div slot="controls" class="flex gap-2 overflow-visible pl-2 pt-2">
      <Button iconButton>
        <Plus />
      </Button>
    </div>
  </Canvas>
</div>
