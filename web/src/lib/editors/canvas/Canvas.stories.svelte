<script>
  import Button from '$lib/components/Button.svelte';
  import Plus from '$lib/components/icons/Plus.svelte';
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import { schemeOranges } from 'd3';
  import BoxToBoxArrow from './BoxToBoxArrow.svelte';
  import Canvas from './Canvas.svelte';
  import CanvasTitledBox from './CanvasTitledBox.svelte';
  import DrawRectangle from './DrawRectangle.svelte';

  let box1 = { x: 30, y: 100, w: 150, h: 150 };
  let box2 = { x: 400, y: 250, w: 250, h: 200 };
  let box3 = { x: 400, y: 550, w: 250, h: 300 };
  let box4 = { x: 30, y: 500, w: 200, h: 200 };

  const headerHeight = 20;
  const rowSize = 16;

  let position = { x: 0, y: 0 };

  $: box12LineStart = { x: box1.x + box1.w, y: box1.y + headerHeight };
  $: box12LineEnd = { x: box2.x, y: box2.y + headerHeight };

  $: box13LineStart = { x: box1.x + box1.w, y: box1.y + headerHeight };
  $: box13LineEnd = { x: box3.x, y: box3.y + headerHeight };

  $: box42LineStart = { x: box4.x + box4.w, y: box4.y + headerHeight };
  $: box42LineEnd = { x: box2.x, y: box2.y + headerHeight + rowSize };

  function clickedAddButton() {
    state = state === 'addingBox' ? 'normal' : 'addingBox';
  }

  let boxes = [];
  function addBox(box) {
    boxes = [
      ...boxes,
      {
        box: {
          x: box.x - position.x,
          y: box.y - position.y,
          w: Math.max(box.w, 150),
          h: Math.max(box.h, 150),
        },
        name: `New Box ${boxes.length + 1}`,
      },
    ];

    state = 'normal';
  }

  let state = 'normal';
  let addButton;
</script>

<svelte:window
  on:keydown={(e) => {
    if (e.key === 'Escape') {
      state = 'normal';
      addButton?.blur();
    }
  }} />

<Meta title="Editors/Canvas" component={Canvas} />

<Template let:args>
  <Canvas
    {...args}
    draggable={args.draggable && state === 'normal'}
    bind:position
    let:position={{ x, y }}>
    <BoxToBoxArrow
      start={{ box: box1, point: box12LineStart, offset: 0 }}
      end={{ box: box2, point: box12LineEnd, offset: 0 }}
      color={schemeOranges[9][3]} />
    <BoxToBoxArrow
      start={{ box: box1, point: box13LineStart, offset: 1 }}
      end={{ box: box3, point: box13LineEnd, offset: 0 }}
      color={schemeOranges[9][3]} />
    <BoxToBoxArrow
      start={{ box: box4, point: box42LineStart, offset: 0 }}
      end={{ box: box2, point: box42LineEnd, offset: 1 }}
      color={schemeOranges[9][7]} />

    <CanvasTitledBox bind:position={box1} dragDeadZone={args.dragDeadZone} name="A moveable box">
      <div class="text-center">Move me!</div>
    </CanvasTitledBox>
    <CanvasTitledBox bind:position={box2} dragDeadZone={args.dragDeadZone} name="With a textarea">
      <textarea class="h-full w-full resize-none">Move me too!</textarea>
    </CanvasTitledBox>
    <CanvasTitledBox bind:position={box3} dragDeadZone={args.dragDeadZone} name="Boring Box">
      <div class="text-center">Another box</div>
    </CanvasTitledBox>
    <CanvasTitledBox bind:position={box4} dragDeadZone={args.dragDeadZone} name="Some other box">
      <div class="text-center">Other Settings</div>
    </CanvasTitledBox>

    {#each boxes as box}
      <CanvasTitledBox bind:position={box.box} name={box.name} dragDeadZone={args.dragDeadZone}>
        <div class="text-center">A box</div>
      </CanvasTitledBox>
    {/each}

    <div slot="controls">
      <div
        class="absolute top-4 left-4 z-50 flex items-center justify-between gap-4 rounded-lg border border-black bg-dgray-100 py-2 pl-4 pr-2 shadow-lg">
        <span class="w-32 whitespace-nowrap">
          X: {x}, Y: {y}
        </span>

        <Button bind:element={addButton} iconButton on:click={clickedAddButton}>
          <Plus />
        </Button>
      </div>

      {#if state === 'addingBox'}
        <DrawRectangle
          on:done={(e) => addBox(e.detail)}
          class="border-2 border-daccent-100 bg-accent-500/25" />
      {/if}

      <div class="absolute bottom-4 left-4">Status</div>
    </div>
  </Canvas>
</Template>

<Story name="Draggable" args={{ draggable: true }} />
<Story name="Draggable with Dead Zone" args={{ draggable: true, dragDeadZone: 10 }} />
<Story name="Wheel Scroll Only" args={{ draggable: false }} />
