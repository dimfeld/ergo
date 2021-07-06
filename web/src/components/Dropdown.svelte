<script lang="ts">
  import { scale } from 'svelte/transition';
  import { cubicIn, cubicOut } from 'svelte/easing';
  import ChevronDown from './icons/ChevronDown.svelte';
  import Button from './Button.svelte';
  import type { SvelteComponent } from 'svelte';
  import type { Position } from './tippy';
  import { showTippy } from './tippy';

  export let open = false;
  export let disabled = false;
  export let position: Position = 'bottom-end';
  export let label: string;
  export let arrow: typeof SvelteComponent | undefined | null | false = ChevronDown;
  export let closeOnClickInside = true;

  let classNames = '';
  export { classNames as class };

  let dropdownButton: HTMLButtonElement;

  $: open = open && !disabled;

  function clicked() {
    if (closeOnClickInside && open) {
      open = false;
    }
  }
</script>

<div class="relative inline-block text-left">
  <div aria-expanded={open} aria-haspopup="true">
    <Button
      bind:element={dropdownButton}
      {disabled}
      class={classNames}
      on:click={() => (open = !open)}
    >
      <slot name="button"
        ><div class="flex space-x-1 items-center">
          <span>{label}</span>
          {#if arrow}<svelte:component this={arrow} class="h-5 w-5" />{/if}
        </div></slot
      >
    </Button>
  </div>

  {#if open && dropdownButton}
    <div
      use:showTippy={{
        trigger: dropdownButton,
        position,
        interactive: true,
        role: 'menu',
        close: () => (open = false),
      }}
      on:click={clicked}
    >
      <div
        in:scale={{ duration: 100, start: 0.95, easing: cubicOut }}
        out:scale={{ duration: 75, start: 0.95, easing: cubicIn }}
        class="py-2 rounded-md shadow-lg bg-white dark:bg-black ring-1 ring-black dark:ring-gray-200 ring-opacity-5 focus:outline-none"
      >
        <slot />
      </div>
    </div>
  {/if}
</div>
