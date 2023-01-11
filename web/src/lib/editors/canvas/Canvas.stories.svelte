<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import Canvas from './Canvas.svelte';
  import CanvasNode from './CanvasNode.svelte';
  import BoxToBoxArrow from './BoxToBoxArrow.svelte';

  import { schemeOranges } from 'd3';

  let box1 = { x: 30, y: 100, w: 150, h: 150 };
  let box2 = { x: 400, y: 250, w: 250, h: 200 };
  let box3 = { x: 400, y: 550, w: 250, h: 300 };
  let box4 = { x: 30, y: 500, w: 200, h: 200 };

  const headerHeight = 20;
  const rowSize = 16;

  $: box12LineStart = { x: box1.x + box1.w, y: box1.y + headerHeight };
  $: box12LineEnd = { x: box2.x, y: box2.y + headerHeight };

  $: box13LineStart = { x: box1.x + box1.w, y: box1.y + headerHeight + rowSize };
  $: box13LineEnd = { x: box3.x, y: box3.y + headerHeight };

  $: box42LineStart = { x: box4.x + box4.w, y: box4.y + headerHeight + rowSize };
  $: box42LineEnd = { x: box2.x, y: box2.y + headerHeight + rowSize };
</script>

<Meta title="Editors/Canvas" component={Canvas} />

<Template let:args>
  <Canvas {...args} let:position={{ x, y }}>
    <BoxToBoxArrow
      start={{ box: box1, point: box12LineStart, offset: 0 }}
      end={{ box: box2, point: box12LineEnd, offset: 0 }}
      color={schemeOranges[9][3]} />
    <BoxToBoxArrow
      start={{ box: box1, point: box13LineStart, offset: 1 }}
      end={{ box: box3, point: box13LineEnd, offset: 0 }}
      color={schemeOranges[9][5]} />
    <BoxToBoxArrow
      start={{ box: box4, point: box42LineStart, offset: 0 }}
      end={{ box: box2, point: box42LineEnd, offset: 1 }}
      color={schemeOranges[9][7]} />

    <CanvasNode bind:position={box1} dragDeadZone={args.dragDeadZone}>
      <div class="text-center">Move me!</div>
    </CanvasNode>
    <CanvasNode bind:position={box2} dragDeadZone={args.dragDeadZone}>
      <textarea class="h-full w-full resize-none">Move me too!</textarea>
    </CanvasNode>
    <CanvasNode bind:position={box3} dragDeadZone={args.dragDeadZone}>
      <div class="text-center">Another box</div>
    </CanvasNode>
    <CanvasNode bind:position={box4} dragDeadZone={args.dragDeadZone}>
      <div class="text-center">Other Settings</div>
    </CanvasNode>

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
