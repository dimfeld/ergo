<script lang="ts">
  import { showTippy } from './tippy';
  import { createEventDispatcher, tick } from 'svelte';
  import XIcon from '$lib/components/icons/X.svelte';
  import CheckIcon from '$lib/components/icons/Check.svelte';
  import { cls } from '$lib/styles';

  const dispatch = createEventDispatcher<{ change: string; input: string }>();

  export let value: string | null | undefined;
  export let editing = false;
  export let placeholder: string | undefined = undefined;
  export let error: string | null | undefined = undefined;
  export let validateOn: 'save' | 'input' = 'input';
  export let validate:
    | ((value: string, existingValue: string) => string | null | undefined)
    | undefined = undefined;
  export let buttonSizeClasses = 'w-4 h-4';

  let classNames: string | undefined;
  export { classNames as class };

  export let editingClasses =
    'border ring-accent-500 border-accent-500 focus:ring-accent-500 focus:border-accent-500';
  export let normalClasses =
    'border cursor-pointer border-transparent ring-accent-500 focus:ring-accent-500 focus:border-transparent hover:border-gray-400';

  $: inputClass = cls('w-full', classNames, editing ? editingClasses : normalClasses);

  let textField: HTMLInputElement;
  let currentInput = value ?? '';
  function handleInput(e: InputEvent) {
    let currentValue = (e.currentTarget as HTMLInputElement)?.value ?? '';

    dispatch('input', currentValue);
    if (validate && validateOn === 'input') {
      error = validate(currentValue, value ?? '');
    }
  }

  function handleKeyUp(e: KeyboardEvent) {
    if (editing) {
      if (e.key === 'Escape') {
        e.stopPropagation();
        e.preventDefault();
        cancel();
      } else if (e.key === 'Enter') {
        e.stopPropagation();
        e.preventDefault();
        save();
      }
    } else {
      if (e.key === 'Enter') {
        startEditing();
        e.stopPropagation();
        e.preventDefault();
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
    error = null;
    currentInput = value ?? '';
  }

  function save() {
    error = validate?.(currentInput, value ?? '');
    if (error) {
      textField?.focus();
      return false;
    }

    editing = false;
    if (value != currentInput) {
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

  $: if (!editing) {
    currentInput = value ?? '';
  }
</script>

<div bind:this={container} class="relative" on:focusout={handleBlur}>
  <input
    bind:this={textField}
    type="text"
    readonly={!editing}
    class={inputClass}
    class:border-red-500={editing && Boolean(error)}
    bind:value={currentInput}
    {placeholder}
    on:input={handleInput}
    on:focus={startEditing}
    on:click={startEditing}
    on:keyup={handleKeyUp}
    title={editing ? '' : 'Click to edit'} />
  {#if editing}
    <div
      class="absolute inset-y-0 right-0 z-10 m-px 
       flex flex-row items-center rounded-r-md bg-white
       pr-2 dark:bg-gray-800">
      <button
        on:click={save}
        class="cursor-pointer rounded px-1 hover:bg-gray-100 dark:hover:bg-gray-700"
        title="OK"><CheckIcon class="inline {buttonSizeClasses}" /></button>
      <button
        on:click={cancel}
        class="cursor-pointer rounded px-1 hover:bg-gray-100 dark:hover:bg-gray-700"
        title="Cancel"><XIcon class="inline {buttonSizeClasses}" /></button>
    </div>
  {/if}

  {#if error}
    <div
      class="rounded-lg bg-red-200 px-4 py-2 dark:bg-red-400 dark:text-black"
      use:showTippy={{ trigger: textField, position: 'bottom' }}>
      {error}
    </div>
  {/if}
</div>
