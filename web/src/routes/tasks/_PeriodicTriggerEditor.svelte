<script lang="ts">
  import { TaskTrigger } from '$lib/api_types';
  import Button from '$lib/components/Button.svelte';
  import DangerButton from '$lib/components/DangerButton.svelte';
  import PlusIcon from '$lib/components/icons/Plus.svelte';
  import InlineEditTextField from '$lib/components/InlineEditTextField.svelte';

  export let trigger: TaskTrigger;

  function defaultNewItem() {
    return {
      isNewItem: true,
      periodic: {
        name: '',
        payload: {},
        enabled: true,
        schedule: { type: 'Cron', data: '' },
      },
    };
  }

  let newItem = defaultNewItem();
  $: periodic = [
    ...(trigger.periodic ?? []).map((periodic) => ({ periodic, isNewItem: false })),
    newItem,
  ];

  function deleteIndex(i) {
    trigger.periodic = [...trigger.periodic.slice(0, i), ...trigger.periodic.slice(i + 1)];
  }

  function addItem(p) {
    trigger.periodic = [...(trigger.periodic || []), p];

    newItem = defaultNewItem();
  }
</script>

<div class="max-h-32 w-96 p-2">
  <p class="text-lg">Periodic Triggers</p>
  <header class="periodic-row mt-2 text-sm font-medium">
    <span>Name</span>
    <span />
  </header>
  <ul class="flex flex-col mt-2 space-y-2">
    {#each periodic as { periodic, isNewItem }, i}
      <li class="periodic-row">
        <InlineEditTextField
          bind:value={periodic.name}
          placeholder={isNewItem ? 'New Trigger Name' : ''}
        />

        <span> Schedule goes here</span>

        {#if isNewItem}
          <Button
            disabled={!periodic.schedule.data}
            on:click={() => addItem(periodic)}
            iconButton={true}
          >
            <PlusIcon />
          </Button>
        {:else}
          <DangerButton on:click={() => deleteIndex(i)}>
            <span slot="title"
              >Delete Trigger <span class="text-gray-700 dark:text-gray-200 font-bold"
                >{periodic.name}</span
              ></span
            >
          </DangerButton>
        {/if}
      </li>
    {/each}
  </ul>
</div>

<style>
  .periodic-row {
    display: grid;
    grid-template-rows: 1;
    grid-template-columns: repeat(2, 1fr) auto;
    column-gap: 1em;
  }
</style>
