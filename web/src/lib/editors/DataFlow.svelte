<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import Plus from '$lib/components/icons/Plus.svelte';
  import Canvas from './canvas/Canvas.svelte';
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
