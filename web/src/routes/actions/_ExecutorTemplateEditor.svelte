<script lang="ts">
  import { TemplateField } from '$lib/api_types';
  import Labelled from '$lib/components/Labelled.svelte';
  import AnyEditor from '$lib/components/AnyEditor.svelte';

  export let field: TemplateField;
  export let value: any;
</script>

<Labelled label="{field.name} - {field.format.type}" help={field.description}>
  <AnyEditor type={field.format.type} {value} on:change />
</Labelled>
