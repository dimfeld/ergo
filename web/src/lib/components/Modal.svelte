<script context="module" lang="ts">
  export type ModalOpener<DIALOGINPUT, DIALOGRESULT> = (
    data: DIALOGINPUT
  ) => Promise<DIALOGRESULT | undefined>;
  export type ModalCloser<DIALOGRESULT> = (result?: DIALOGRESULT) => void;
</script>

<script lang="ts">
  import { portal } from 'svelte-portal/src/Portal.svelte';
  import { focus } from 'focus-svelte';
  import { fade } from 'svelte/transition';

  type DIALOGINPUT = $$Generic;
  type DIALOGRESULT = $$Generic;

  interface $$Slots {
    default: { close: ModalCloser<DIALOGRESULT>; data: DIALOGINPUT };
    opener: { open: ModalOpener<DIALOGINPUT, DIALOGRESULT> };
    backdrop: { close: ModalCloser<DIALOGRESULT> };
  }

  export let target = 'body';
  export let backdrop = true;
  export let trapFocus = true;
  export let closeOnEsc = true;
  export let closeOnClickOutside = true;

  let promiseResolve: ((value?: DIALOGRESULT) => void) | undefined;
  let openInput: DIALOGINPUT;

  export function open(data: DIALOGINPUT): Promise<DIALOGRESULT | undefined> {
    if (promiseResolve) {
      // Resolve any existing promise in case something else tries to open this modal while it's already open.
      promiseResolve();
    }

    openInput = data;
    let p = new Promise<DIALOGRESULT | undefined>((resolve) => (promiseResolve = resolve));
    return p;
  }

  export function close(value?: DIALOGRESULT) {
    promiseResolve?.(value);
    promiseResolve = undefined;
  }

  function closeOnEscAction(_node: HTMLElement) {
    if (!closeOnEsc) {
      return {};
    }

    const handleKeydown = (e: KeyboardEvent) => {
      if (closeOnEsc && e.key === 'Escape') {
        close();
      }
    };

    document.addEventListener('keyup', handleKeydown, { passive: true });
    return {
      destroy: () => document.removeEventListener('keyup', handleKeydown, { passive: true }),
    };
  }
</script>

<slot name="opener" {open} />

{#if promiseResolve}
  <!-- Extra wrapping div to keep Svelte from erroring -->
  <div class="hidden">
    <div
      use:portal={target}
      class="absolute inset-0 h-screen w-screen grid grid-cols-1 grid-rows-1 place-items-center z-1000"
    >
      {#if backdrop}
        <slot name="backdrop" {close}>
          <div
            class="absolute inset-0 bg-black bg-opacity-25 dark:bg-opacity-75"
            in:fade={{ duration: 150 }}
            out:fade={{ duration: 100 }}
            on:click={() => closeOnClickOutside && close()}
          />
        </slot>
      {/if}
      <div
        use:closeOnEscAction
        use:focus={{ enabled: trapFocus }}
        class="z-10 bg-gray-50 dark:bg-gray-900 p-4 rounded-lg border border-gray-200 dark:border-gray-400 shadow-xl"
      >
        <slot {close} data={openInput} />
      </div>
    </div>
  </div>
{/if}
