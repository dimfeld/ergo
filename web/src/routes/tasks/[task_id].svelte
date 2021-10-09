<script context="module" lang="ts">
  import { loadFetch } from '$lib/api';
  import type { Load } from '@sveltejs/kit';

  export const load: Load = async function load({ fetch, page }) {
    fetch = loadFetch(fetch);

    if (page.params.task_id === 'new') {
      return {
        props: {},
      };
    }

    let task: TaskResult = await fetch(`/api/tasks/${page.params.task_id}`).then((r) => r.json());

    return {
      props: {
        task,
      },
    };
  };
</script>

<script lang="ts">
  import { goto, invalidate } from '$app/navigation';
  import { getStores, page } from '$app/stores';
  import TaskTriggerList from './_TaskTriggerList.svelte';
  import TaskActionList from './_TaskActionList.svelte';
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import TextField from '$lib/components/TextField.svelte';
  import type { TaskResult } from '$lib/api_types';
  import { getHeaderTextStore } from '$lib/header';
  import { onDestroy } from 'svelte';

  import ScriptEditor from '$lib/editors/Script.svelte';
  import StateMachineEditor from '$lib/editors/StateMachine.svelte';
  import { baseData } from '$lib/data';
  import apiClient from '$lib/api';
  import { TaskConfigValidator } from 'ergo-wasm';
  import initWasm from '$lib/wasm';

  export let task: TaskResult = defaultTask();

  const taskEditors = {
    Js: ScriptEditor,
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

  let getEditorState: () => Promise<{ compiled: any; source: any }>;
  async function save() {
    let taskType = task.source?.type;
    if (!taskType) {
      return;
    }

    let { source, compiled } = await getEditorState();
    task.source = source;
    task.compiled = compiled;

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
        <input type="text" bind:value={task.alias} placeholder="None" class="ml-2" />
      </p>
    </div>
    <p>Description: {task.description || ''}</p>
    <p>Modified {task.modified}</p>
  </Card>

  <Card class="mt-4 flex flex-col">
    <p class="section-header">Actions</p>
    <TaskActionList
      taskId={task.task_id}
      bind:taskActions={task.actions}
      on:change={() => (task.actions = task.actions)}
    />
  </Card>

  <Card class="mt-4 flex flex-col">
    <p class="section-header">Triggers</p>
    <TaskTriggerList
      taskId={task.task_id}
      bind:triggers={task.triggers}
      on:change={() => (task.triggers = task.triggers)}
    />
  </Card>

  <Card class="flex flex-col flex-grow mt-4 h-[64em]">
    {#if taskSource}
      <div class="flex-1 grid grid-rows-1 grid-cols-1 place-items-stretch">
        <svelte:component
          this={taskEditors[taskSource.type]}
          bind:getState={getEditorState}
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
      <Button on:click={() => initializeSource('Js')}>Script</Button>
      <Button on:click={() => initializeSource('Flowchart')}>FlowChart</Button>
    </p>
  </Card>
</div>

<style lang="postcss">
  .section-header {
    @apply font-bold text-gray-700 dark:text-gray-300;
  }
</style>
