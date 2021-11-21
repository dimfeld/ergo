<script lang="ts">
  import { portal } from 'svelte-portal';
  import { fade } from 'svelte/transition';

  type DIALOGINPUT = $$Generic;
  type DIALOGRESULT = $$Generic;

  interface $$Slots {
    default: { finish: (value?: DIALOGRESULT) => void; data: DIALOGINPUT };
  }

  let promiseResolve: ((value?: DIALOGRESULT) => void) | undefined;
  let showInput: DIALOGINPUT;

  export function show(data: DIALOGINPUT): Promise<DIALOGRESULT | undefined> {
    if (promiseResolve) {
      // Resolve any existing promise in case something else tries to open this modal while it's already open.
      promiseResolve();
    }

    showInput = data;
    let p = new Promise<DIALOGRESULT | undefined>((resolve) => (promiseResolve = resolve));
    return p;
  }

  export function hide(value?: DIALOGRESULT) {
    promiseResolve?.(value);
    promiseResolve = undefined;
  }
</script>

{#if promiseResolve}
  <div
    use:portal
    class="relative h-screen w-screen grid grid-cols-1 grid-rows-1 place-items-center z-1000"
  >
    <div
      class="absolute inset-0 bg-black bg-opacity-25"
      transition:fade={{ duration: 150 }}
      on:click={() => hide()}
    />
    <div class="z-10">
      <slot finish={hide} data={showInput} />
    </div>
  </div>
{/if}
