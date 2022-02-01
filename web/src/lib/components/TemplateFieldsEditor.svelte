<!--
  @component
  Edit the fields in a TemplateFields object
-->
<script lang="ts">
  import { TemplateFieldFormat, TemplateFields } from '$lib/api_types';
  import Button from './Button.svelte';
  import Checkbox from './Checkbox.svelte';
  import Labelled from './Labelled.svelte';
  import StringListEditor from './StringListEditor.svelte';
  import { keyHandler } from '../keyhandlers';
  import pascalCase from 'just-pascal-case';

  export let fields: TemplateFields;

  function addTemplateInput(e: Event) {
    e.preventDefault();
    if (!newTemplateInputName) {
      return;
    }

    fields = [
      ...fields,
      {
        name: newTemplateInputName,
        format:
          newTemplateInputType === 'choice'
            ? { type: 'choice', choices: [], min: 1, max: 1, default: [] }
            : {
                type: newTemplateInputType,
                default: undefined,
              },
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
    <li class:pt-4={i > 0} class="pb-4">
      <Labelled
        big={true}
        label={template_field.name}
        help={pascalCase(template_field.format.type)}
      >
        <div class="flex flex-col space-y-4">
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
          {:else if template_field.format.type === 'object'}
            <Checkbox bind:value={template_field.format.nested} label="Allow Nested Objects" />
          {/if}
          <Labelled label="Description">
            <input class="w-full" type="text" bind:value={template_field.description} />
          </Labelled>
          <!-- TODO Add support for default values -->
          <div class="flex items-center justify-between">
            <Checkbox bind:value={template_field.optional} label="Optional" />
            <Button style="danger" on:click={() => removeTemplateField(i)}>Delete</Button>
          </div>
        </div>
      </Labelled>
    </li>
  {/each}
  <li class="flex items-center space-x-2 pt-4">
    <span>Add new</span>
    <select bind:value={newTemplateInputType}>
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
      type="text"
      bind:value={newTemplateInputName}
      on:keydown={keyHandler(['Enter'], addTemplateInput)}
    />
    <Button on:click={addTemplateInput}>Add</Button>
  </li>
</ul>
