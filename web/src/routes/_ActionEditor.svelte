<script lang="ts">
  import { Action } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import Labelled from '$lib/components/Labelled.svelte';
  import { ModalCloser } from '$lib/components/Modal.svelte';
  import { baseData } from '$lib/data';

  const { executors } = baseData();

  export let data: Action;
  export let close: ModalCloser<Action>;

  $: executor = $executors.get(data.executor_id);

  let actionCategories = {}; // TODO

  function handleSubmit() {
    if (!data.name) {
      // TODO error message
      return;
    }

    close(data);
  }
</script>

<form on:submit|preventDefault={handleSubmit} class="max-w-[95vw] max-h-[95vw]">
  <div class="overflow-y-auto flex flex-col space-y-4">
    <Labelled label="Name"><input type="text" class="w-full" bind:value={data.name} /></Labelled>
    <Labelled label="Description"
      ><input type="text" class="w-full" bind:value={data.description} /></Labelled
    >
    <Labelled label="Category">
      <select class="w-full " bind:value={data.action_category_id}>
        {#each Object.entries(actionCategories) as [id, name]}
          <option value={id}>{name}</option>
        {/each}
      </select>
    </Labelled>
    <div class="flex space-x-4">
      <Labelled class="w-1/2" label="Executor">
        <select class="w-full" bind:value={data.executor_id}>
          {#each Array.from($executors.values()) as info}
            <option>{info.name}</option>
          {/each}
        </select>
      </Labelled>
      <Labelled class="w-1/2" label="Timeout (seconds)">
        <input
          class="w-full"
          type="number"
          bind:value={data.timeout}
          placeholder="Timeout in Seconds"
        />
      </Labelled>
    </div>
    <Labelled label="Executor Template">
      <!-- TODO script/template toggle -->
      <ul>
        {#each executor?.template_fields || [] as field, i}
          <li>
            <Labelled label={field.name} help={field.description}>
              {JSON.stringify(field)}
            </Labelled>
          </li>
        {/each}
      </ul>
    </Labelled>
    <Labelled label="Inputs">
      <!-- action inputs -->
    </Labelled>
    <Labelled label="Accounts">
      <!-- account types -->
    </Labelled>
    <!-- postprocess script -->
  </div>
  <div class="flex justify-end space-x-2">
    <Button type="submit" style="primary">OK</Button>
    <Button on:click={() => close()}>Cancel</Button>
  </div>
</form>
