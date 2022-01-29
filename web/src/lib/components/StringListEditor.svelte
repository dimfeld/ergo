<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Button from './Button.svelte';
  import Plus from './icons/Plus.svelte';
  import X from './icons/X.svelte';

  const dispatch = createEventDispatcher();
  const notify = () => dispatch('change', values);

  export let values: string[];

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

<ol class="flex flex-col space-y-2">
  {#each values as value, i}
    <li class="flex items-stretch space-x-2">
      <input
        type="text"
        {value}
        on:input={(e) => updateIndex(e, i)}
        class="rounded-none border-0 border-b border-gray-200 py-0 dark:border-gray-500"
      /><Button iconButton aria-label="Delete" on:click={() => remove(i)}><X /></Button>
    </li>
  {/each}
  <li class="flex items-stretch space-x-2">
    <input
      type="text"
      class="rounded-none border-0 border-b border-gray-200 py-0 dark:border-gray-500"
      on:keydown={handleKeydown}
    /><Button iconButton aria-label="Add new line" on:click={(e) => addNew(e.target)}
      ><Plus /></Button
    >
  </li>
</ol>
