<script lang="ts">
  import { invalidate } from '$app/navigation';
  import apiClient from '$lib/api';
  import { Input } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import Modal, { ModalCloser, ModalOpener } from '$lib/components/Modal.svelte';
  import { baseData } from '$lib/data';
  import { getHeaderTextStore } from '$lib/header';
  import makeClone from 'rfdc';
  import InputEditor from './_InputEditor.svelte';
  const clone = makeClone();
  const { inputs } = baseData();
  getHeaderTextStore().set(['Inputs']);

  const api = apiClient();
  let openDialog: ModalOpener<Input | undefined, Input>;
  async function editInput(input: Input | undefined) {
    let result = await openDialog(clone(input));
    if (result) {
      if (input) {
        await api.put(`inputs/${input.input_id}`, {
          json: result,
        });
      } else {
        await api.post(`inputs`, { json: result });
      }

      invalidate('/api/inputs');
    }
  }
</script>

<ul class="space-y-4">
  {#each Array.from($inputs.values()) as input (input.input_id)}
    <li>
      <Card>
        <p>
          {input.name}{#if input.description} &mdash; {input.description}{/if}
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
