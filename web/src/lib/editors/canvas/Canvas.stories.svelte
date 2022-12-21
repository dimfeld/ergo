<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import Canvas from './Canvas.svelte';
</script>

<Meta title="Editors/Canvas" component={Canvas} />

<Template let:args>
  <Canvas {...args} let:position={{ x, y }}>
    <div
      class="h-16 w-32 rounded-xl border-black bg-red-400 px-2 py-4 text-center shadow-xl"
      style:transform="translate(30px, 100px) scale(1)">
      Move me!
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

      <div class="absolute bottom-4 left-4">Status</div>
    </div>
  </Canvas>
</Template>

<Story name="Draggable" args={{ draggable: true }} />
<Story name="Draggable with Dead Zone" args={{ draggable: true, dragDeadZone: 10 }} />
<Story name="Wheel Scroll Only" args={{ draggable: false }} />
