<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import Canvas from './Canvas.svelte';
  import CanvasNode from './CanvasNode.svelte';
  import BoxToBoxArrow from './BoxToBoxArrow.svelte';

  let pos1 = { x: 30, y: 100 };
  let size1 = { x: 150, y: 150 };
  let pos2 = { x: 400, y: 250 };
  let size2 = { x: 250, y: 200 };
</script>

<Meta title="Editors/Canvas" component={Canvas} />

<Template let:args>
  <Canvas {...args} let:position={{ x, y }}>
    <CanvasNode bind:position={pos1} bind:size={size1}>
      <div class="text-center">Move me!</div>
    </CanvasNode>
    <CanvasNode bind:position={pos2} bind:size={size2}>
      <textarea class="h-full w-full resize-none">Move me too!</textarea>
    </CanvasNode>

    <BoxToBoxArrow
      from={{ x: pos1.x, y: pos1.y, w: size1.x, h: size1.y }}
      to={{ x: pos2.x, y: pos2.y, w: size2.x, h: size2.y }} />

    <div slot="controls">
      <div
        class="absolute top-4 left-4 flex h-16 w-24 flex-col items-start justify-center gap-2 rounded-lg border border-black bg-dgray-100 px-4 py-2 shadow-lg">
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
