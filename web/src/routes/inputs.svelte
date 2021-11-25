<script lang="ts">
  import { invalidate } from '$app/navigation';
  import apiClient from '$lib/api';
  import { Input } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import Modal, { ModalCloser, ModalOpener } from '$lib/components/Modal.svelte';
  import { new_input_id } from 'ergo-wasm';
  import initWasm from '$lib/wasm';
  import { baseData } from '$lib/data';
  import { getHeaderTextStore } from '$lib/header';
  import makeClone from 'rfdc';
  import InputEditor from './_InputEditor.svelte';
  const clone = makeClone();
  const { inputs } = baseData();
  getHeaderTextStore().set(['Inputs']);

  initWasm();

  const api = apiClient();
  let openDialog: ModalOpener<Input | undefined, Input>;
  async function editInput(input: Input | undefined) {
    let result = await openDialog(clone(input) ?? newInput());
    if (result) {
      await api.put(`api/inputs/${result.input_id}`, {
        json: result,
      });

      invalidate('/api/inputs');
    }
  }

  function newInput(): Input {
    let inputId = new_input_id();
    return {
      input_id: inputId,
      name: '',
      payload_schema: {
        $schema: 'http://json-schema.org/draft-07/schema',
        $id: 'http://ergo.dev/inputs/${inputId}',
        type: 'object',
        required: [],
        properties: {},
        additionalProperties: true,
      },
    };
  }
</script>

<Button class="self-start" on:click={() => editInput(undefined)}>New Input</Button>

<ul class="space-y-4 mt-4">
  {#each Array.from($inputs.values()) as input (input.input_id)}
    <li>
      <Card>
        <p>
          {input.name}
          {#if input.description} &mdash; {input.description}{/if}
        </p>

        <Button on:click={() => editInput(input)}>Edit</Button>
        <ul>
          {#each Object.entries(input.payload_schema.properties) as [field, fieldType] (field)}
            <li>
              <span class="text-gray-800 dark:text-gray-200 font-medium">{field}</span>: {fieldType.type}
              {#if fieldType.format}
                in {fieldType.format} format{/if}
            </li>
          {/each}
        </ul>
      </Card>
    </li>
  {/each}
</ul>

<Modal bind:open={openDialog} let:data let:close>
  <InputEditor {close} {data} />
</Modal>
