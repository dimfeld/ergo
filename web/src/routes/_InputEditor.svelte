<script lang="ts">
  import { Input } from '$lib/api_types';
  import { ModalCloser } from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';

  import Labelled from '$lib/components/Labelled.svelte';

  export let data: Input;
  export let close: ModalCloser<Input>;

  function handleSubmit() {
    // TODO input validation
    close(data);
  }
</script>

<form on:submit|preventDefault={() => handleSubmit()} class="w-[40rem] flex flex-col space-y-4">
  <Labelled label="Name"><input type="text" class="w-full" bind:value={data.name} /></Labelled>
  <Labelled label="Description"
    ><input type="text" class="w-full" bind:value={data.description} /></Labelled
  >
  <Labelled label="Schema">
    <!-- TODO payload schema editor -->
  </Labelled>
  <div class="flex justify-end space-x-2">
    <Button type="submit" style="primary">OK</Button>
    <Button on:click={() => close()}>Cancel</Button>
  </div>
</form>
