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

    dispatch('change', newValue);
    value = newValue;
  }
</script>

{#if format.type === 'string'}
  <input class="w-full" type="text" value={value ?? ''} on:input={(e) => notify(e.target.value)} />
{:else if format.type === 'string_array'}
  <StringListEditor values={value ?? []} on:change={(e) => notify(e.detail)} />
{:else if format.type === 'object'}
  {#if format.nested}
    <ObjectOrJsonTextEditor value={value ?? {}} on:change={(e) => notify(e.detail)} />
  {:else}
    <ObjectEditor value={value ?? {}} on:change={(e) => notify(e.detail)} />
  {/if}
{:else if format.type === 'boolean'}
  <label>
    <!-- TODO this should be some sort of tri-state when optional is true -->
    <input type="checkbox" checked={value ?? false} on:change={(e) => notify(e.target.checked)} />
    <span class="font-medium text-sm">Enabled?</span>
  </label>
{:else if format.type === 'integer' || format.type === 'float'}
  <input
    type="number"
    step={format.type === 'float' ? 0.01 : 1}
    {value}
    on:input={(e) => notifyNumber(e.target.valueAsNumber)}
  />
{/if}
