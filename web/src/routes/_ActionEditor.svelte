<script lang="ts">
  import { Action } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import Labelled from '$lib/components/Labelled.svelte';
  import { ModalCloser } from '$lib/components/Modal.svelte';

  export let data: Action;
  export let close: ModalCloser<Action>;

  let actionCategories = {}; // TODO
  let executors = {}; // TODO get this list from the api

  function handleSubmit() {
    if (!data.name) {
      // TODO error message
      return;
    }

    close(data);
  }
</script>

<form on:submit|preventDefault={handleSubmit} class="w-[40rem] flex flex-col space-y-4">
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
        {#each Object.entries(executors) as [id, name]}
          <option value={id}>{name}</option>
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
    <!-- action inputs -->
    <!-- executor template editor -->
    <!-- account types -->
    <!-- postprocess script -->
  </div>
  <div class="flex justify-end space-x-2">
    <Button type="submit" style="primary">OK</Button>
    <Button on:click={() => close()}>Cancel</Button>
  </div>
</form>
