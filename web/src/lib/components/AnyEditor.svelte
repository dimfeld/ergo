<script lang="ts">
  import StringListEditor from './StringListEditor.svelte';
  import { createEventDispatcher } from 'svelte';
  import ObjectEditor from './ObjectEditor.svelte';

  const dispatch = createEventDispatcher();
  const notify = (value: any) => dispatch('change', value);

  export let value: any;
  export let type:
    | 'string'
    | 'string_array'
    | 'object'
    | 'boolean'
    | 'integer'
    | 'float'
    | 'choice';

  if (value === undefined) {
    switch (type) {
      case 'string':
        value = '';
        break;
      case 'string_array':
        value = [];
        break;
      case 'object':
        value = {};
        break;
      case 'boolean':
        value = false;
        break;
    }
  }
</script>

{#if type === 'string'}
  <input class="w-full" type="text" {value} on:input={(e) => notify(e.target.value)} />
{:else if type === 'string_array'}
  <StringListEditor values={value} on:change={(e) => notify(e.detail)} />
{:else if type === 'object'}
  <ObjectEditor {value} on:change={(e) => notify(e.detail)} />
{:else if type === 'boolean'}
  <label>
    <input type="checkbox" checked={value} on:change={(e) => notify(e.target.checked)} />
    <span class="font-medium text-sm">Enabled?</span>
  </label>
{:else if type === 'integer' || type === 'float'}
  <input
    type="number"
    step={type === 'float' ? 0.01 : 1}
    {value}
    on:input={(e) => notify(e.target.value)}
  />
{/if}
