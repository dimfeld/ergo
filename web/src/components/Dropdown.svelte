<script lang="ts">
  import tippy from 'tippy.js/headless';
  import 'tippy.js/themes/light.css';
  import 'tippy.js/animations/shift-away.css';
  import 'tippy.js/dist/tippy.css';
  import ChevronDown from './icons/ChevronDown.svelte';
  import type { SvelteComponent } from 'svelte';

  export let open = false;
  export let disabled = false;
  export let position: 'top' | 'bottom' | 'left' | 'right' = 'bottom';
  export let label: string;
  export let arrow: typeof SvelteComponent | undefined | null | false = ChevronDown;

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
  <slot name="button"
    >{label}
    {#if arrow}<svelte:component this={arrow} />{/if}</slot
  >
</button>

{#if open}
  <div use:showTippy>
    <slot />
  </div>
{/if}
