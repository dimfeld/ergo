<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import Canvas from './Canvas.svelte';
</script>

<Meta title="Editors/Canvas" component={Canvas} />

<Template />

<Story name="Position">
  <Canvas let:position={{ x, y }}>
    <div
      class="h-16 w-32 rounded-xl border-black bg-red-400 px-2 py-4 text-center shadow"
      style:transform="translate(30px, 100px) scale(1)">
      Scroll me!
    </div>
    <div slot="controls">
      <div
        class="absolute top-4 left-4 flex h-16 w-24 flex-col items-start justify-center gap-2 rounded-lg border border-black bg-gray-100 px-4 py-2 shadow-lg">
        <span>
          X: {x}
        </span>
        <span>
          Y: {y}
        </span>
      </div>
    </div>
  </Canvas>
</Story>
