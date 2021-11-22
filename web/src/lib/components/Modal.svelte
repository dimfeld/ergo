<script context="module" lang="ts">
  export type ModalOpener<DIALOGINPUT, DIALOGRESULT> = (
    data: DIALOGINPUT
  ) => Promise<DIALOGRESULT | undefined>;
  export type ModalCloser<DIALOGRESULT> = (result?: DIALOGRESULT) => void;
</script>

<script lang="ts">
  import { portal } from 'svelte-portal';
  import { focus } from 'focus-svelte';
  import { fade } from 'svelte/transition';

  type DIALOGINPUT = $$Generic;
  type DIALOGRESULT = $$Generic;

  interface $$Slots {
    default: { close: ModalCloser<DIALOGRESULT>; data: DIALOGINPUT };
    backdrop: { close: ModalCloser<DIALOGRESULT> };
  }

  export let backdrop = true;
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

  function closeOnEscAction() {
    if (!closeOnEsc) {
      return {};
    }

    const handleKeydown = (e: KeyboardEvent) => {
      if (closeOnEsc && e.key === 'Escape') {
        close();
      }
    };

    document.addEventListener('keydown', handleKeydown, { passive: true });
    return {
      destroy: () => document.removeEventListener('keydown', handleKeydown, { passive: true }),
    };
  }
</script>

{#if promiseResolve}
  <div
    use:portal
    class="relative h-screen w-screen grid grid-cols-1 grid-rows-1 place-items-center z-1000"
  >
    {#if backdrop}
      <slot name="backdrop" {close}>
        <div
          class="absolute inset-0 bg-black bg-opacity-25"
          in:fade={{ duration: 150 }}
          out:fade={{ duration: 100 }}
          on:click={() => closeOnClickOutside && close()}
        />
      </slot>
    {/if}
    <div class="z-10" use:closeOnEscAction use:focus={{ enabled: true }}>
      <slot {close} data={openInput} />
    </div>
  </div>
{/if}
