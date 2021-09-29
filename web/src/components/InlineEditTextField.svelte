<script lang="ts">
  import { showTippy } from './tippy';
  import { createEventDispatcher, tick } from 'svelte';
  import XIcon from '$lib/components/icons/X.svelte';
  import CheckIcon from '$lib/components/icons/Check.svelte';

  const dispatch = createEventDispatcher<{ change: string }>();

  export let value: string;
  export let editing = false;
  export let placeholder: string | undefined = undefined;
  export let error: string | null | undefined = undefined;
  export let validateOn: 'save' | 'input' = 'input';
  export let validate: ((value: string) => string | null | undefined) | undefined = undefined;

  export let editingClasses =
    'border ring-accent-500 border-accent-500 focus:ring-accent-500 focus:border-accent-500';
  export let normalClasses =
    'border cursor-pointer border-transparent ring-accent-500 focus:ring-accent-500 focus:border-transparent hover:border-gray-400';

  $: classNames = editing ? editingClasses : normalClasses;

  let textField: HTMLInputElement;
  let currentInput = value;
  function handleInput(e: InputEvent) {
    if (validate && validateOn === 'input') {
      error = validate((e.currentTarget as HTMLInputElement)?.value);
    }
  }

  function handleKeyUp(e: KeyboardEvent) {
    if (editing) {
      if (e.key === 'Escape') {
        cancel();
      } else if (e.key === 'Enter') {
        save();
      }
    } else {
      if (e.key === 'Enter') {
        startEditing();
      }
    }
  }

  async function startEditing() {
    if (!editing) {
      editing = true;
      await tick();
      textField?.focus();
    }
  }

  function cancel() {
    editing = false;
    currentInput = value;
  }

  function save() {
    error = validate?.(currentInput);
    if (error) {
      textField?.focus();
      return false;
    }

    editing = false;
    if (value !== currentInput) {
      value = currentInput;
      dispatch('change', value);
    }
    return true;
  }

  function handleBlur(e: FocusEvent) {
    if (e.relatedTarget && container?.contains(e.relatedTarget as Element)) {
      // This is the OK or Cancel button, so don't leave editing mode.
      return;
    }

    save();
  }

  let container: HTMLDivElement;
</script>

<div bind:this={container} class="relative" on:focusout={handleBlur}>
  <input
    bind:this={textField}
    type="text"
    readonly={!editing}
    class="w-full {classNames}"
    class:border-red-500={editing && Boolean(error)}
    bind:value={currentInput}
    {placeholder}
    on:input={handleInput}
    on:focus={startEditing}
    on:click={startEditing}
    on:keyup={handleKeyUp}
    title={editing ? '' : 'Click to edit'}
  />
  {#if editing}
    <div
      class="absolute inset-y-0 right-0 m-px
       pr-2 z-10 flex flex-row items-center
       bg-white dark:bg-gray-800"
    >
      <button
        on:click={save}
        class="px-1 cursor-pointer rounded hover:bg-gray-100 dark:hover:bg-gray-700"
        title="OK"><CheckIcon class="h-4 w-4 inline" /></button
      >
      <button
        on:click={cancel}
        class="px-1 cursor-pointer rounded hover:bg-gray-100 dark:hover:bg-gray-700"
        title="Cancel"><XIcon class="h-4 w-4 inline" /></button
      >
    </div>
  {/if}

  {#if error}
    <div
      class="bg-red-200 rounded-lg px-4 py-2"
      use:showTippy={{ trigger: textField, position: 'bottom' }}
    >
      {error}
    </div>
  {/if}
</div>
