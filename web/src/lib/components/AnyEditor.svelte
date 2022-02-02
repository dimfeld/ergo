<script lang="ts">
  import StringListEditor from './StringListEditor.svelte';
  import { createEventDispatcher } from 'svelte';
  import ObjectEditor from './ObjectEditor.svelte';
  import isEmpty from 'just-is-empty';
  import ObjectOrJsonTextEditor from './ObjectOrJsonTextEditor.svelte';
  import { TemplateFieldFormat } from '$lib/api_types';

  const dispatch = createEventDispatcher();

  export let value: any;
  export let optional = false;
  export let format: TemplateFieldFormat;
  /* If editing an object, expect `value` to be parseable JSON and then stringify it when setting `value` */
  export let objectAsString = false;

  const notify = (newValue: any) => {
    if (optional) {
      switch (format.type) {
        case 'string':
        case 'string_array':
        case 'object':
        case 'choice':
          if (isEmpty(newValue)) {
            newValue = null;
          }
          break;
      }
    }

    if (format.type === 'object' && objectAsString) {
      newValue = JSON.stringify(newValue);
    }

    value = newValue;
    dispatch('change', newValue);
  };

  function notifyNumber(newValue: number | null) {
    if (Number.isNaN(value) || newValue === null) {
      if (optional) {
        newValue = null;
      } else {
        return;
      }
    } else if (format.type === 'integer') {
      newValue = Math.trunc(newValue);
    }

    value = newValue;
    dispatch('change', newValue);
  }

  function notifyChoice(newValue: string | string[]) {
    value = Array.isArray(newValue) ? newValue : [newValue];
    dispatch('change', value);
  }

  $: multiple = format.type === 'choice' && format.max > 1;

  function makeObjectValue(v) {
    return objectAsString ? JSON.parse(v || '{}') : v || {};
  }
</script>

{#if format.type === 'string'}
  <input class="w-full" type="text" value={value ?? ''} on:input={(e) => notify(e.target.value)} />
{:else if format.type === 'string_array'}
  <StringListEditor values={value ?? []} on:change={(e) => notify(e.detail)} />
{:else if format.type === 'object'}
  {#if format.nested}
    <ObjectOrJsonTextEditor value={makeObjectValue(value)} on:change={(e) => notify(e.detail)} />
  {:else}
    <ObjectEditor value={makeObjectValue(value)} on:change={(e) => notify(e.detail)} />
  {/if}
{:else if format.type === 'boolean'}
  <label>
    <!-- TODO this should be some sort of tri-state when optional is true -->
    <input type="checkbox" checked={value ?? false} on:change={(e) => notify(e.target.checked)} />
    <span class="font-medium text-sm">Enabled?</span>
  </label>
{:else if format.type === 'integer' || format.type === 'float'}
  <input
    class="w-full"
    type="number"
    step={format.type === 'float' ? 0.01 : 1}
    {value}
    on:input={(e) => notifyNumber(e.target.valueAsNumber)}
  />
{:else if format.type === 'choice'}
  <select
    class="w-full"
    {multiple}
    value={multiple ? value : value?.[0]}
    on:change={(e) => notifyChoice(e.target.value)}
  >
    {#each format.choices as choice}
      <option>{choice}</option>
    {/each}
  </select>
{/if}
