<script lang="ts">
  import { showTippy } from './tippy';
  import { tick } from 'svelte';
  import XIcon from '^/components/icons/X.svelte';
  import CheckIcon from '^/components/icons/Check.svelte';

  export let value: string;
  export let editing = false;
  export let error: string | null | undefined = undefined;
  export let validateOn: 'save' | 'input' = 'input';
  export let validate: ((value: string) => string | null | undefined) | undefined = undefined;

  export let editingClasses = 'border focus:ring-accent-500 focus:border-accent-500';
  export let normalClasses =
    "border-none cursor-pointer hover:border hover:border-gray-200 dark:hover:border-gray-700';";

  $: classNames = editing ? editingClasses : normalClasses;

  let textField: HTMLInputElement;
  let currentInput = value;
  function handleInput(e: InputEvent) {
    if (validate && validateOn === 'input') {
      error = validate(e.currentTarget.value);
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
      return false;
    }

    editing = false;
    value = currentInput;
    return true;
  }

  function handleBlur() {
    if (!save()) {
      cancel();
    }
  }
</script>

<div class="relative">
  <input
    bind:this={textField}
    type="text"
    readonly={!editing}
    class="bg-white dark-bg-gray-800 sm:text-sm {classNames}"
    class:border-red-500={editing && Boolean(error)}
    bind:value={currentInput}
    on:input={handleInput}
    on:keyup={handleKeyUp}
    on:focus={startEditing}
    on:blur={handleBlur}
    on:click={startEditing}
  />
  {#if editing}
    <div
      class="absolute inset-y-0 right-0
       pr-2 z-10 flex flex-row space-x-2
       bg-white dark:bg-gray-800"
    >
      <span on:click={cancel}><XIcon /></span>
      <span on:click={save}><CheckIcon /></span>
    </div>
  {/if}

  {#if error}
    <div class="bg-red-200 rounded-lg px-4 py-2" use:showTippy={{ position: 'bottom' }}>
      {error}
    </div>
  {/if}
</div>
