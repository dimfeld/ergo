<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';
  import sorter from 'sorters';
  import Button from './Button.svelte';
  import Plus from './icons/Plus.svelte';
  import X from './icons/X.svelte';

  const dispatch = createEventDispatcher();
  const notify = () => dispatch('change', value);

  export let value: Record<string, any>;

  let keyTextfields: Record<string, HTMLInputElement> = {};
  let newKeyField: HTMLElement;

  function updateKey(e: InputEvent, oldKey: string) {
    let current = document.activeElement ?? {};
    let { selectionStart, selectionEnd } = current as HTMLInputElement;

    let newKey = e.target?.value ?? '';
    if (newKey && newKey !== oldKey) {
      value[newKey] = value[oldKey];
      delete value[oldKey];
    }
    tick().then(() => {
      let newElement = keyTextfields[newKey];
      if (newElement) {
        newElement.focus();
        if (selectionStart !== undefined) {
          newElement.selectionStart = selectionStart;
          newElement.selectionEnd = selectionEnd;
        }
      }
    });
    notify();
  }

  function updateValue(key: string, keyValue: string) {
    value[key] = keyValue;
    notify();
  }

  function remove(key: string) {
    delete value[key];
    value = value;
    notify();
  }

  function addNew() {
    value[newKey] = newValue;
    newKey = '';
    newValue = '';
    newKeyField?.focus();
    notify();
  }

  function handleValueKeydown(e: KeyboardEvent, key: string) {
    if (e.target.value && (e.key === 'Enter' || e.key === 'Tab')) {
      e.preventDefault();
      addNew();
    }
  }

  let newKey = '';
  let newValue = '';
</script>

<ol class="flex w-full flex-col space-y-2">
  {#each Object.entries(value).sort(sorter((x) => x[0])) as [key, value], i (key)}
    <li class="flex items-stretch ">
      <input
        type="text"
        bind:this={keyTextfields[key]}
        aria-label="Key {key}"
        value={key}
        on:input={(e) => updateKey(e, key)}
        class="mr-4 w-full flex-1 py-0"
      />
      <slot name="value" {value} update={(newValue) => updateValue(key, newValue)}>
        <input
          type="text"
          aria-label="Value for key {key}"
          {value}
          on:input={(e) => updateValue(key, e.target.value)}
          class="mr-2 w-full flex-1 py-0"
        />
      </slot>
      <Button iconButton aria-label="Delete key {key}" on:click={() => remove(key)}><X /></Button>
    </li>
  {/each}
  <li class="flex items-stretch">
    <input
      type="text"
      class="mr-4 w-full border-gray-300 py-0 dark:border-gray-700"
      placeholder="Key"
      aria-label="New item key"
      bind:value={newKey}
      bind:this={newKeyField}
    />
    <input
      type="text"
      class="mr-2 w-full border-gray-300 py-0 dark:border-gray-700"
      placeholder="Value"
      aria-label="New item value"
      bind:value={newValue}
      on:keydown={handleValueKeydown}
    />
    <Button iconButton aria-label="Add new item" on:click={addNew}><Plus /></Button>
  </li>
</ol>
