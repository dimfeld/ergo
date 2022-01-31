<script lang="ts">
  import { showTippy } from './tippy';
  import { scale } from 'svelte/transition';

  let classNames: string = '';
  export { classNames as class };

  let showing = false;
</script>

<div
  use:showTippy={{
    on: 'mouseenter focus',
    showOnCreate: false,
    onTrigger: () => (showing = true),
    onUntrigger: () => (showing = false),
  }}
  class:hidden={!showing}
>
  {#if showing}
    <div
      in:scale={{ duration: 100 }}
      class="max-w-lg rounded-lg border border-black bg-dwhite py-2 px-3 text-base font-normal text-dgray-800 shadow-lg dark:border-gray-400 {classNames}"
    >
      <slot />
    </div>
  {/if}
</div>
