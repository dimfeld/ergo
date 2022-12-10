<!--
  @component
  Edit the fields in a TemplateFields object
-->
<script lang="ts">
  import type { TemplateField, TemplateFieldFormat, TemplateFields } from '$lib/api_types';
  import Button from './Button.svelte';
  import Checkbox from './Checkbox.svelte';
  import Labelled from './Labelled.svelte';
  import StringListEditor from './StringListEditor.svelte';
  import { keyHandler } from '../keyhandlers';
  import pascalCase from 'just-pascal-case';
  import AnyEditor from './AnyEditor.svelte';

  export let fields: TemplateFields;

  function addTemplateInput(e: Event) {
    e.preventDefault();
    if (!newTemplateInputName) {
      return;
    }

    let format: TemplateField['format'];
    switch (newTemplateInputType) {
      case 'choice':
        format = { type: 'choice', choices: [], min: 1, max: 1, default: [] };
        break;
      case 'string':
        format = { type: newTemplateInputType, default: '' };
        break;
      case 'object':
        format = { type: newTemplateInputType, default: '{}' };
        break;
      case 'integer':
      case 'float':
        format = { type: newTemplateInputType, default: 0 };
        break;
      case 'string_array':
        format = { type: 'string_array', default: [] };
        break;
      case 'boolean':
        format = { type: 'boolean', default: false };
        break;
    }

    fields = [
      ...fields,
      {
        name: newTemplateInputName,
        format,
        optional: true,
        description: '',
      },
    ];

    newTemplateInputName = '';
  }

  function removeTemplateField(index: number) {
    fields = [...fields.slice(0, index), ...fields.slice(index + 1)];
  }

  let newTemplateInputType: TemplateFieldFormat['type'] = 'string';
  let newTemplateInputName = '';
</script>

<ul class="flex flex-col divide-y divide-dgray-300">
  {#each fields as template_field, i (template_field.name)}
    <li class:pt-8={i > 0} class="pb-8">
      <Labelled
        big={true}
        label={template_field.name}
        help={pascalCase(template_field.format.type)}
      >
        <div class="flex flex-col space-y-4">
          <Labelled label="Description">
            <input class="w-full" type="text" bind:value={template_field.description} />
          </Labelled>
          {#if template_field.format.type === 'choice'}
            <StringListEditor
              bind:values={template_field.format.choices}
              placeholder="Add New Choice"
            />
            <div class="flex justify-between space-x-4">
              <Labelled class="w-full" label="Choose at least">
                <input class="w-full" type="number" bind:value={template_field.format.min} />
              </Labelled>
              <Labelled class="w-full" label="Choose at most">
                <input class="w-full" type="number" bind:value={template_field.format.max} />
              </Labelled>
            </div>
          {/if}
          <Labelled label="Default Value">
            <AnyEditor
              optional={false}
              objectAsString={true}
              format={template_field.format}
              bind:value={template_field.format.default}
            />
          </Labelled>
          <div class="flex items-center">
            <Checkbox bind:value={template_field.optional} label="Optional" class="mr-4" />
            {#if template_field.format.type === 'object'}
              <Checkbox
                bind:value={template_field.format.nested}
                label="Allow Nested Objects"
                class="ml-8 mr-4"
              />
            {/if}
            <Button class="ml-auto" style="danger" on:click={() => removeTemplateField(i)}
              >Delete</Button
            >
          </div>
        </div>
      </Labelled>
    </li>
  {/each}
  <li class="flex items-center space-x-2 pt-8">
    <span>Add new</span>
    <select class="h-8 py-0" bind:value={newTemplateInputType}>
      <option>string</option>
      <option value="string_array">string array</option>
      <option value="object">object</option>
      <option>boolean</option>
      <option>choice</option>
      <option>integer</option>
      <option>float</option>
    </select>
    <span>named</span>
    <input
      class="h-8 py-0"
      type="text"
      bind:value={newTemplateInputName}
      on:keydown={keyHandler(['Enter'], addTemplateInput)}
    />
    <Button size="sm" on:click={addTemplateInput}>Add</Button>
  </li>
</ul>
