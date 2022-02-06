<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Button from './Button.svelte';
  import Plus from './icons/Plus.svelte';
  import X from './icons/X.svelte';

  const dispatch = createEventDispatcher();
  const notify = () => dispatch('change', values);

  export let values: string[];
  export let placeholder = 'Add New Item';

  /** If set, use a select box and limit choices to the values herein. */
  export let possible: string[] | undefined = undefined;

  function updateIndex(e: InputEvent, i: number) {
    let value = e.target?.value ?? '';
    values = [...values.slice(0, i), value, ...values.slice(i + 1)];
    notify();
  }

  function remove(i: number) {
    values = [...values.slice(0, i), ...values.slice(i + 1)];
    notify();
  }

  let newValue = '';
  function addNew() {
    if (newValue) {
      values = [...values, newValue];
      notify();
      newValue = '';
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.target.value && (e.key === 'Enter' || e.key === 'Tab')) {
      e.preventDefault();
      addNew();
    }
  }
</script>

<ol class="flex w-full flex-col space-y-2">
  {#each values as value, i}
    <li class="flex items-stretch space-x-2">
      {#if possible}
        <select
          {placeholder}
          class="w-full border-gray-300"
          {value}
          aria-label="New item"
          on:change={(e) => updateIndex(e, i)}
        >
          {#each possible as option}
            <option>{option}</option>
          {/each}
        </select>
      {:else}
        <input type="text" {value} on:input={(e) => updateIndex(e, i)} class="w-full py-0" />
      {/if}
      <Button iconButton aria-label="Delete" on:click={() => remove(i)}><X /></Button>
    </li>
  {/each}
  <li class="flex items-stretch space-x-2">
    {#if possible}
      <select
        class="w-full border-gray-300"
        bind:value={newValue}
        aria-label="New item"
        on:keydown={handleKeydown}
      >
        <option value="" />
        {#each possible as option}
          <option>{option}</option>
        {/each}
      </select>
    {:else}
      <input
        type="text"
        {placeholder}
        bind:value={newValue}
        aria-label="New item"
        class="w-full border-gray-300 py-0 dark:border-gray-700"
        on:keydown={handleKeydown}
      />
    {/if}
    <Button iconButton aria-label="Add new item" on:click={addNew}><Plus /></Button>
  </li>
</ol>
