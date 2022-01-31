<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Button from './Button.svelte';
  import Plus from './icons/Plus.svelte';
  import X from './icons/X.svelte';

  const dispatch = createEventDispatcher();
  const notify = () => dispatch('change', values);

  export let values: string[];
  export let placeholder = 'Add New Item';

  function updateIndex(e: InputEvent, i: number) {
    let value = e.target?.value ?? '';
    values = [...values.slice(0, i), value, ...values.slice(i + 1)];
    notify();
  }

  function remove(i: number) {
    values = [...values.slice(0, i), ...values.slice(i + 1)];
    notify();
  }

  function addNew(target: HTMLInputElement) {
    let value = target.value;
    if (value) {
      values = [...values, value];
      target.value = '';
      notify();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.target.value && (e.key === 'Enter' || e.key === 'Tab')) {
      e.preventDefault();
      addNew(e.target);
    }
  }
</script>

<ol class="flex w-full flex-col space-y-2">
  {#each values as value, i}
    <li class="flex items-stretch space-x-2">
      <input type="text" {value} on:input={(e) => updateIndex(e, i)} class="w-full py-0" /><Button
        iconButton
        aria-label="Delete"
        on:click={() => remove(i)}><X /></Button
      >
    </li>
  {/each}
  <li class="flex items-stretch space-x-2">
    <input
      type="text"
      {placeholder}
      aria-label="New item"
      class="w-full border-gray-300 py-0 dark:border-gray-700"
      on:keydown={handleKeydown}
    /><Button iconButton aria-label="Add new item" on:click={(e) => addNew(e.target)}
      ><Plus /></Button
    >
  </li>
</ol>
