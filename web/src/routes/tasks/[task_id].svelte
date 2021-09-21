<script context="module" lang="ts">
  import { createApiClient } from '^/api';

  /**
   * @type {import('@sveltejs/kit').Load}
   */
  export async function load({ fetch, page }) {
    const client = createApiClient();

    if (page.params.task_id === 'new') {
      return {
        props: {},
      };
    }

    let task = await client.get(`/api/tasks/${page.params.task_id}`).json<TaskResult>();

    return {
      props: {
        task,
      },
    };
  }
</script>

<script lang="ts">
  import { goto, invalidate } from '$app/navigation';
  import { getStores, page } from '$app/stores';
  import Button from '^/components/Button.svelte';
  import TextField from '^/components/TextField.svelte';
  import type { TaskResult } from '^/api_types';
  import { getHeaderTextStore } from '^/header';

  import ScriptEditor from '^/editors/Script.svelte';
  import StateMachineEditor from '^/editors/StateMachine.svelte';
  import { baseData } from '^/data';
  import apiClient from '^/api';

  export let task: TaskResult = defaultTask();

  const taskEditors = {
    Script: ScriptEditor,
    StateMachine: StateMachineEditor,
  };

  const { page } = getStores();
  const { inputs, actions } = baseData();
  const client = apiClient();

  function defaultTask(): TaskResult {
    let created = new Date().toISOString();
    return {
      task_id: '',
      name: '',
      source: null,
      compiled: null,
      actions: {},
      triggers: {},
      state: null,
      enabled: false,
      modified: created,
      created,
      task_template_version: 0,
      alias: null,
      description: null,
    };
  }

  $: newTask = $page.params.task_id === 'new';

  function revert() {
    if (newTask) {
      task = defaultTask();
    } else {
      invalidate(`/api/tasks/${$page.params.task_id}`);
    }
  }

  async function save() {
    if (newTask) {
      let result = await client.post(`/api/tasks`, { json: task }).json<{ task_id: string }>();
      task.task_id = result.task_id;
      goto(result.task_id, { replaceState: true, noscroll: true, keepfocus: true });
    } else {
      await client.put(`/api/tasks/${$page.params.task_id}`, { json: task });
    }
  }

  const headerText = getHeaderTextStore();
  $: headerText.set(['Tasks', task.name || 'New Task']);

  function initializeSource(type: string) {
    task.source = { type, data: null };
  }

  $: taskSource = task.source || task.compiled;

  $: taskActions = Object.entries(task.actions).map(([localId, action]) => {
    return {
      localId,
      taskAction: action,
      action: $actions.get(action.action_id),
    };
  });

  $: taskTriggers = Object.entries(task.triggers).map(([localId, trigger]) => {
    return {
      localId,
      trigger,
      input: $inputs.get(trigger.input_id),
    };
  });
</script>

<div class="flex flex-col flex-grow">
  <section class="flex flex-row justify-end space-x-4">
    <Button on:click={revert}>Revert</Button>
    <Button style="primary" on:click={save}>Save</Button>
  </section>
  <section
    class="mt-2 flex flex-col space-y-2 w-full p-2 rounded border border-gray-200 dark:border-gray-400 shadow-md"
  >
    <div class="flex w-full justify-between">
      <p class="text-sm">
        ID: <span class:text-gray-500={!task.task_id}>{task.task_id || 'New Task'}</span>
      </p>
      <p>
        <span class="font-medium text-sm text-gray-700 dark:text-gray-300">Alias</span>
        <TextField type="text" bind:value={task.alias} placeholder="None" class="ml-2" />
      </p>
    </div>
    <p>Description: {task.description || ''}</p>
    <p>Modified {task.modified}</p>
    <p class="font-medium text-gray-700 dark:text-gray-300">Actions</p>
    {#each taskActions as action (action.localId)}
      <p>{JSON.stringify(action)}</p>
    {/each}
    <p class="font-medium text-gray-700 dark:text-gray-300">Triggers</p>
    {#each taskTriggers as trigger (trigger.localId)}
      <p>{JSON.stringify(trigger)}</p>
    {/each}
  </section>

  <section class="flex flex-col flex-grow mt-4">
    {#if taskSource}
      <div class="flex-grow grid grid-rows-1 grid-cols-1 place-items-stretch">
        <svelte:component
          this={taskEditors[taskSource.type]}
          source={task.source?.data}
          compiled={task.compiled?.data}
        />
      </div>
    {/if}
    <p>
      {#if taskSource}Change the task type{:else}Select a task type{/if}
    </p>
    <p class="flex space-x-2">
      <button on:click={() => initializeSource('StateMachine')}>State Machine</button>
      <button on:click={() => initializeSource('Script')}>Script</button>
      <button on:click={() => initializeSource('Flowchart')}>FlowChart</button>
    </p>
  </section>
</div>
