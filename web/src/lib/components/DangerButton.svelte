<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  import X from './icons/X.svelte';
  import Dropdown from './Dropdown.svelte';
  import Button from './Button.svelte';

  export let title = 'Delete';
  export let dropdownOpen = false;

  const dispatch = createEventDispatcher<{ click: void }>();

  async function clickYes() {
    dropdownOpen = false;
    dispatch('click');
  }
</script>

<Dropdown bind:open={dropdownOpen} closeOnClickInside={false}>
  <div slot="button"><Button style="danger" iconButton={true}><X /></Button></div>

  <div class="flex flex-col py-2 px-4 w-max max-w-md">
    <span class="text-lg text-gray-300 font-medium"><slot name="title">{title}</slot></span>
    <span class="text-sm pb-4">Are you sure?</span>
    <div class="flex flex-row justify-end space-x-2">
      <Button style="primary" on:click={clickYes}>Yes</Button>
      <Button on:click={() => (dropdownOpen = false)}>No</Button>
    </div>
  </div>
</Dropdown>
