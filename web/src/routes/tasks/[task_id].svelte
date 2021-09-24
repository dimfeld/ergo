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
  import Card from '^/components/Card.svelte';
  import TextField from '^/components/TextField.svelte';
  import type { TaskResult } from '^/api_types';
  import { getHeaderTextStore } from '^/header';
  import { onDestroy } from 'svelte';

  import ScriptEditor from '^/editors/Script.svelte';
  import StateMachineEditor from '^/editors/StateMachine.svelte';
  import { baseData } from '^/data';
  import apiClient from '^/api';
  import { TaskConfigValidator } from 'ergo-wasm';
  import initWasm from '^/wasm';

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

  let validator: TaskConfigValidator | undefined;
  let wasmLoaded = false;
  initWasm().then(() => (wasmLoaded = true));
  $: if (wasmLoaded) {
    validator?.free();
    validator = new TaskConfigValidator($actions, $inputs, task.triggers, task.actions);
  }

  onDestroy(() => {
    wasmLoaded = false;
    validator?.free();
  });
</script>

<div class="flex flex-col flex-grow">
  <section class="flex flex-row justify-end space-x-4">
    <!-- TODO add confirmation dropdown -->
    <Button on:click={revert}>Revert</Button>
    <Button style="primary" on:click={save}>Save</Button>
  </section>
  <Card class="mt-2 flex flex-col">
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
  </Card>

  <Card class="mt-4 flex flex-col">
    <p class="font-bold text-gray-700 dark:text-gray-300">Actions</p>
    <div id="task-actions">
      <span class="header">Local ID</span>
      <span class="header">Local Action Name</span>
      <span class="header">Action Name</span>
      {#each taskActions as action (action.localId)}
        <span>{action.localId}</span>
        <span>{action.taskAction.name}</span>
        <span>{action.action.name}</span>
      {/each}
    </div>
  </Card>

  <Card class="mt-4 flex flex-col">
    <p class="font-bold text-gray-700 dark:text-gray-300">Triggers</p>
    <div id="task-triggers">
      <span class="header">Local ID</span>
      <span class="header">Local Trigger Name</span>
      <span class="header">Input Name</span>
      {#each taskTriggers as trigger (trigger.localId)}
        <span>{trigger.localId}</span>
        <span>{trigger.trigger.name}</span>
        <span>{trigger.input.name}</span>
      {/each}
    </div>
  </Card>

  <Card class="flex flex-col flex-grow mt-4 h-[64em]">
    {#if taskSource}
      <div class="flex-1 grid grid-rows-1 grid-cols-1 place-items-stretch">
        <svelte:component
          this={taskEditors[taskSource.type]}
          source={task.source?.data}
          compiled={task.compiled?.data}
          {validator}
        />
      </div>
    {/if}
    <p class="mt-4">
      {#if taskSource}Change the task type{:else}Select a task type{/if}
    </p>
    <p class="flex space-x-2">
      <Button on:click={() => initializeSource('StateMachine')}>State Machine</Button>
      <Button on:click={() => initializeSource('Script')}>Script</Button>
      <Button on:click={() => initializeSource('Flowchart')}>FlowChart</Button>
    </p>
  </Card>
</div>

<style lang="postcss">
  #task-actions,
  #task-triggers {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: auto;
  }

  #task-actions .header,
  #task-triggers .header {
    @apply font-medium text-gray-800 dark:text-gray-200;
  }
  .section-header {
    @apply font-bold text-gray-700 dark:text-gray-300;
  }
</style>
