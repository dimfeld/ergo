<script lang="ts">
  import StringListEditor from './StringListEditor.svelte';
  import { createEventDispatcher } from 'svelte';
  import ObjectEditor from './ObjectEditor.svelte';
  import isEmpty from 'just-is-empty';

  const dispatch = createEventDispatcher();

  export let value: any;
  export let optional = false;
  export let type:
    | 'string'
    | 'string_array'
    | 'object'
    | 'boolean'
    | 'integer'
    | 'float'
    | 'choice';

  const notify = (newValue: any) => {
    if (optional) {
      switch (type) {
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
    if (Number.isNaN(value)) {
      if (optional) {
        newValue = null;
      } else {
        return;
      }
    } else if (type === 'integer' && typeof newValue === 'number') {
      newValue = Math.trunc(newValue);
    }

    dispatch('change', newValue);
    value = newValue;
  }
</script>

{#if type === 'string'}
  <input class="w-full" type="text" value={value ?? ''} on:input={(e) => notify(e.target.value)} />
{:else if type === 'string_array'}
  <StringListEditor values={value ?? []} on:change={(e) => notify(e.detail)} />
{:else if type === 'object'}
  <ObjectEditor value={value ?? {}} on:change={(e) => notify(e.detail)} />
{:else if type === 'boolean'}
  <label>
    <!-- TODO this should be some sort of tri-state when optional is true -->
    <input type="checkbox" checked={value ?? false} on:change={(e) => notify(e.target.checked)} />
    <span class="font-medium text-sm">Enabled?</span>
  </label>
{:else if type === 'integer' || type === 'float'}
  <input
    type="number"
    step={type === 'float' ? 0.01 : 1}
    {value}
    on:input={(e) => notifyNumber(e.target.valueAsNumber)}
  />
{/if}
