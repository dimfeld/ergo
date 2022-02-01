<!-- @component Provide values to be filled into a template -->
<script lang="ts">
  import { ScriptOrTemplate, TemplateFields } from '$lib/api_types';
  import Labelled from './Labelled.svelte';
  import Editor from '$lib/editors/Editor.svelte';
  import AnyEditor from './AnyEditor.svelte';
  import pascalCase from 'just-pascal-case';

  export let fields: TemplateFields;
  export let values: ScriptOrTemplate;

  // TODO: Support scripts. This will require generating typescript types from the executor's template for both the
  // inputs and outputs.
  $: valuesByName =
    values.t === 'Template'
      ? Object.fromEntries(
          values.c.map(([name, value], index) => {
            return [
              name,
              {
                value,
                index,
              },
            ];
          })
        )
      : {};

  function updateExecutorTemplateValue(name: string, value: any) {
    if (values.t === 'Template') {
      let templateValueIndex = values.c.findIndex((v) => v[0] === name);
      if (templateValueIndex >= 0) {
        if (value === null) {
          // Remove the item from the template
          values.c = [
            ...values.c.slice(0, templateValueIndex),
            ...values.c.slice(templateValueIndex + 1),
          ];
        } else {
          values.c[templateValueIndex][1] = value;
        }
      } else {
        values.c = [...values.c, [name, value]];
      }
    }
  }
</script>

<!-- TODO script/template toggle -->
<ul class="flex flex-col space-y-4">
  {#each fields || [] as field}
    <li>
      <Labelled
        label={field.name}
        help="{pascalCase(field.format.type)} &mdash; {field.description}"
      >
        {#if field.name === 'script' && field.format.type === 'string'}
          <!-- Gross hardcoded case but it's the only one for now :) -->
          <Editor
            format="js"
            contents={valuesByName[field.name]?.value}
            notifyOnChange={true}
            on:change={(e) => updateExecutorTemplateValue(field.name, e.detail)}
          />
        {:else}
          <AnyEditor
            optional={field.optional}
            format={field.format}
            value={valuesByName[field.name]?.value}
            on:change={(e) => updateExecutorTemplateValue(field.name, e.detail)}
          />
        {/if}
      </Labelled>
    </li>
  {/each}
</ul>
