<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import Canvas from './Canvas.svelte';
  import CanvasNode from './CanvasNode.svelte';
</script>

<Meta title="Editors/Canvas" component={Canvas} />

<Template let:args>
  <Canvas {...args} let:position={{ x, y }}>
    <CanvasNode position={{ x: 30, y: 100 }} size={{ x: 150, y: 150 }}>
      <div class="text-center">Move me!</div>
    </CanvasNode>
    <CanvasNode position={{ x: 200, y: 100 }} size={{ x: 150, y: 150 }}>
      <div class="text-center">Move me too!</div>
    </CanvasNode>
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
