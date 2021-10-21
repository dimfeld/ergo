<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { TaskResult, TaskTriggerInput } from '../../api_types';
  import { baseData } from '../../data';
  import sorter from 'sorters';
  import initWasm from '$lib/wasm';
  import { new_task_trigger_id } from 'ergo-wasm';
  import Button from '$lib/components/Button.svelte';
  import PlusIcon from '$lib/components/icons/Plus.svelte';
  import DangerButton from '$lib/components/DangerButton.svelte';
  import InlineEditTextField from '$lib/components/InlineEditTextField.svelte';
  import Dropdown from '../../components/Dropdown.svelte';
  import Clock from '../../components/icons/Clock.svelte';

  const dispatch = createEventDispatcher<{ change: void }>();
  const notify = () => dispatch('change');

  export let taskId: string;
  export let triggers: Record<string, TaskTriggerInput>;

  initWasm();

  function noop() {}

  function updateKey(oldKey: string, newKey: string) {
    triggers[newKey] = triggers[oldKey];
    delete triggers[oldKey];
    notify();
  }

  function deleteKey(key: string) {
    delete triggers[key];
    notify();
  }

  const { inputs } = baseData();
  $: triggerRows = [
    ...Object.entries(triggers).map(([localId, trigger]) => {
      return {
        localId,
        trigger,
        input: $inputs.get(trigger.input_id),
        isNewItem: false,
      };
    }),
    newItem,
  ];

  function newItemTemplate() {
    let input = $inputs.values().next().value;
    return {
      localId: '',
      trigger: {
        input_id: input?.input_id,
        name: '',
        description: null,
        periodic: [],
      },
      input,
      isNewItem: true,
    };
  }

  let newItem = newItemTemplate();

  async function addItem(trigger) {
    if (!trigger.localId) {
      return;
    }

    await initWasm();

    console.log({ newTrigger: trigger, triggers });
    triggers[trigger.localId] = {
      ...trigger.trigger,
      task_trigger_id: new_task_trigger_id(),
      task_id: taskId,
    };
    newItem = newItemTemplate();
    notify();
  }

  $: inputTypes = Array.from($inputs.values())
    .map((input) => ({ id: input.input_id, name: input.name }))
    .sort(sorter('name'));

  function validateId(value: string, existing: string) {
    if (value === existing) {
      return;
    }

    if (!value) {
      return 'ID is required';
    }

    if (value in triggers) {
      return 'IDs must be unique';
    }
  }

  const getKeyChangeHandler = (trigger) =>
    trigger.isNewItem
      ? ({ detail: newValue }) => (newItem.localId = newValue)
      : ({ detail: newValue }) => updateKey(trigger.localId, newValue);
  const notifyHandler = (trigger) => (trigger.isNewItem ? noop : notify);
</script>

<div id="task-triggers">
  <span class="header">Trigger ID</span>
  <span class="header">Trigger Name</span>
  <span class="header">Input Type</span>
  <span class="header" />
  <span class="header" />
  {#each triggerRows as trigger (trigger.localId)}
    <InlineEditTextField
      value={trigger.localId}
      placeholder={trigger.isNewItem ? 'New Trigger ID' : ''}
      validateOn="input"
      validate={validateId}
      on:change={getKeyChangeHandler(trigger)}
    />
    <InlineEditTextField
      bind:value={trigger.trigger.name}
      placeholder={trigger.isNewItem ? 'New Trigger Name' : ''}
      on:change={notifyHandler(trigger)}
    />
    <select bind:value={trigger.trigger.input_id} on:change={notifyHandler(trigger)}>
      {#each inputTypes as { id, name } (id)}
        <option value={id}>{name}</option>
      {/each}
    </select>

    <Dropdown arrow={null}>
      <div slot="button">
        <Button iconButton={true}>
          <div
            title="Schedule"
            class:text-gray-400={!trigger.trigger.periodic?.length}
            class:dark:text-gray-600={!trigger.trigger.periodic?.length}
          >
            <Clock />
          </div>
        </Button>
      </div>

      <div class="w-64 p-2">Periodic triggers coming soon!</div>
    </Dropdown>

    {#if trigger.isNewItem}
      <Button disabled={!trigger.localId} on:click={() => addItem(trigger)} iconButton={true}>
        <PlusIcon />
      </Button>
    {:else}
      <DangerButton on:click={() => deleteKey(trigger.localId)}>
        <span slot="title"
          >Delete Trigger <span class="text-gray-700 dark:text-gray-200 font-bold"
            >{trigger.trigger.name || trigger.localId}</span
          ></span
        >
      </DangerButton>
    {/if}
  {/each}
</div>

<style lang="postcss">
  #task-triggers {
    display: grid;
    grid-template-columns: repeat(3, 1fr) repeat(2, auto);
    grid-template-rows: auto;
    column-gap: 1em;
    row-gap: 1em;
    align-items: center;
  }

  .header {
    @apply font-medium text-gray-800 dark:text-gray-200;
  }
</style>
