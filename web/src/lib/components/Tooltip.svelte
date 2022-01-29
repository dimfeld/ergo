<script lang="ts">
  import { showTippy } from './tippy';

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
  class="rounded-lg border border-black bg-white py-2 px-3 shadow-lg dark:border-gray-400 dark:bg-black {classNames}"
  class:hidden={!showing}
>
  <slot />
</div>
