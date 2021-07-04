<script lang="ts">
  import tippy from 'tippy.js/headless';
  import 'tippy.js/themes/light.css';
  import 'tippy.js/animations/shift-away.css';
  import 'tippy.js/dist/tippy.css';

  export let open = false;
  export let disabled = false;
  export let position: 'top' | 'bottom' | 'left' | 'right' = 'bottom';
  export let label: string;
  export let buttonArrow = true;

  let dropdownButton: HTMLButtonElement;

  function showTippy(node: HTMLDivElement) {
    let tippyInstance = tippy(dropdownButton, {
      interactive: true,
      hideOnClick: 'toggle',
      trigger: 'manual',
      maxWidth: 'none',
      placement: position,
      popperOptions: {
        modifiers: [{ name: 'flip' }, { name: 'preventOverflow' }],
      },
      render(_instance) {
        return { popper: node };
      },
    });

    tippyInstance.show();

    return {
      destroy() {
        tippyInstance.destroy();
      },
    };
  }
</script>

<button bind:this={dropdownButton} type="button" {disabled} on:click={() => (open = !open)}>
  <slot name="button">{label}</slot>
</button>

{#if open}
  <div use:showTippy>
    <slot />
  </div>
{/if}
