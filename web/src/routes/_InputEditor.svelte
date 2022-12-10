<script lang="ts">
  import type { Input } from '$lib/api_types';
  import type { ModalCloser } from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import { stringFormats } from '$lib/json_schema';

  import Labelled from '$lib/components/Labelled.svelte';
  import InlineEditTextField from '$lib/components/InlineEditTextField.svelte';
  import { tick } from 'svelte';

  export let data: Input;
  export let close: ModalCloser<Input>;

  function handleSubmit() {
    // TODO input validation
    close(data);
  }

  function validateKey(newKey: string, existing: string) {
    if (newKey == existing) {
      return;
    }

    if (!newKey && existing) {
      return 'Field name must not be blank';
    }

    if (newKey in data.payload_schema.properties) {
      return 'Field name must be unique';
    }
  }

  function updateKey(newKey: string, oldKey: string) {
    data.payload_schema.properties[newKey] = data.payload_schema.properties[oldKey];
    delete data.payload_schema.properties[oldKey];
  }

  // This is overly simple but works to get started.
  const fieldTypes = ['string', 'number', 'integer', 'boolean', 'object', 'array'];

  let newFieldName = '';
  let newFieldType = 'string';
  let newFieldFormat: string | undefined = undefined;
</script>

<form on:submit|preventDefault={() => handleSubmit()} class="w-[40rem] flex flex-col space-y-4">
  <Labelled label="Name"><input type="text" class="w-full" bind:value={data.name} /></Labelled>
  <Labelled label="Description"
    ><input type="text" class="w-full" bind:value={data.description} /></Labelled
  >
  <Labelled label="Schema">
    <div class="flex space-x-4 label">
      <span class="w-1/3">Name</span>
      <span class="w-1/3">Type</span>
      <span class="w-1/3">Format</span>
    </div>
    <ul class="mt-2 flex flex-col space-y-4">
      {#each Object.entries(data.payload_schema.properties) as [field, fieldType] (field)}
        <li class="flex space-x-4">
          <div class="w-1/3">
            <InlineEditTextField
              value={field}
              validate={validateKey}
              on:change={({ detail }) => updateKey(detail, field)}
            />
          </div>
          <select class="w-1/3" bind:value={fieldType.type}>
            {#each fieldTypes as type}
              <option>{type}</option>
            {/each}
          </select>
          <select
            class="w-1/3"
            disabled={fieldType.type !== 'string'}
            bind:value={fieldType.format}
          >
            <option value={undefined}>Any</option>
            {#each stringFormats as format}
              <option>{format}</option>
            {/each}
          </select>
        </li>
      {/each}
      <li class="flex space-x-4">
        <div class="w-1/3">
          <InlineEditTextField
            bind:value={newFieldName}
            validate={validateKey}
            placeholder="New Field Name"
            on:change={({ detail }) => {
              data.payload_schema.properties[detail] = {
                type: newFieldType,
                format: newFieldType == 'string' ? newFieldFormat : undefined,
              };
              tick().then(() => (newFieldName = ''));
            }}
          />
        </div>
        <select class="w-1/3" bind:value={newFieldType}>
          {#each fieldTypes as type}
            <option>{type}</option>
          {/each}
        </select>
        <select class="w-1/3" disabled={newFieldType !== 'string'} bind:value={newFieldFormat}>
          <option value={undefined}>Any</option>
          {#each stringFormats as format}
            <option>{format}</option>
          {/each}
        </select>
      </li>
    </ul>
  </Labelled>
  <div class="flex justify-end space-x-2">
    <Button type="submit" style="primary">OK</Button>
    <Button on:click={() => close()}>Cancel</Button>
  </div>
</form>
