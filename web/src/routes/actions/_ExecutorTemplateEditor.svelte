<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { TemplateField } from '$lib/api_types';
  import Labelled from '$lib/components/Labelled.svelte';
  import StringListEditor from '$lib/components/StringListEditor.svelte';

  const dispatch = createEventDispatcher();
  const notify = (value) => dispatch('change', value);

  export let field: TemplateField;
  export let value: any;
</script>

<Labelled label="{field.name} - {field.format.type}" help={field.description}>
  {#if field.format.type === 'string'}
    <input class="w-full" type="text" {value} on:input={(e) => notify(e.target.value)} />
  {:else if field.format.type === 'string_array'}
    <StringListEditor {value} on:change={(e) => notify(e.detail)} />
  {/if}
</Labelled>
