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
  import Modal, { ModalOpener } from '$lib/components/Modal.svelte';
  import type { TaskAction, TaskResult, TaskTrigger } from '$lib/api_types';
  import { getHeaderTextStore } from '$lib/header';
  import { onDestroy } from 'svelte';
  import makeClone from 'rfdc';
  const clone = makeClone();

  import ScriptEditor from '$lib/editors/Script.svelte';
  import StateMachineEditor from '$lib/editors/StateMachine.svelte';
  import { baseData } from '$lib/data';
  import apiClient from '$lib/api';
  import { TaskConfigValidator } from 'ergo-wasm';
  import initWasm from '$lib/wasm';
  import Labelled from '../../lib/components/Labelled.svelte';
  import Pencil from '../../lib/components/icons/Pencil.svelte';

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

  let showNewTaskDialog: ModalOpener<void, string>;
  $: if (newTask && showNewTaskDialog && !task.source) {
    initializeSource();
  }

  async function initializeSource() {
    let type = await showNewTaskDialog();
    if (type) {
      task.source = { type, data: null };
    } else {
      goto('/tasks');
    }
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

  interface TaskActionEditorData {
    taskActionId: string | null;
    action: TaskAction;
  }
  let openTaskActionEditor: ModalOpener<TaskActionEditorData | undefined, TaskActionEditorData>;
  async function editTaskAction(taskActionId: string | null) {
    let result = await openTaskActionEditor({ taskActionId, action: task.actions[taskActionId] });
    if (result) {
      if (taskActionId && result.taskActionId !== taskActionId) {
        delete task.actions[taskActionId];
      }

      task.actions[result.taskActionId] = result.action;
    }
  }

  interface TaskTriggerEditorData {
    taskTriggerId: string | null;
    trigger: TaskTrigger;
  }
  let openTaskTriggerEditor: ModalOpener<TaskTriggerEditorData | undefined, TaskTriggerEditorData>;
  async function editTaskTrigger(taskTriggerId: string | null) {
    let result = await openTaskTriggerEditor({
      taskTriggerId,
      trigger: clone(task.triggers[taskTriggerId]),
    });
    if (result) {
      if (taskTriggerId && result.taskTriggerId !== taskTriggerId) {
        delete task.triggers[taskTriggerId];
      }

      task.triggers[result.taskTriggerId] = result.trigger;
    }
  }
</script>

<div class="flex flex-col flex-grow">
  <section class="flex flex-row justify-end space-x-4">
    <!-- TODO add confirmation dropdown -->
    <Button on:click={revert}>Revert</Button>
    <Button style="primary" on:click={save}>Save</Button>
  </section>
  <Card class="mt-2 flex flex-col space-y-4">
    <div class="flex w-full justify-between space-x-4">
      <Labelled label="Name" class="w-full"
        ><input class="w-full" type="text" bind:value={task.name} /></Labelled
      >
      <Labelled label="Alias">
        <input type="text" bind:value={task.alias} placeholder="None" />
      </Labelled>
    </div>
    <Labelled label="Description"
      ><input type="text" class="w-full" bind:value={task.description} /></Labelled
    >
    <div class="flex space-x-4 justify-between">
      <p class="text-sm whitespace-nowrap">
        ID: <span class:text-gray-500={!task.task_id}>{task.task_id || 'New Task'}</span>
      </p>
      <p class="text-sm">Modified {task.modified}</p>
    </div>
  </Card>

  <Card class="mt-4 flex flex-col">
    <p class="section-header">Actions</p>

    <div class="w-full task-item-list">
      <span class="font-medium">Local ID</span>
      <span class="font-medium">Description</span>
      <span class="font-medium">Action Type</span>
      <span />
      {#each Object.entries(task.actions) as [taskActionId, taskAction] (taskActionId)}
        <span>{taskActionId}</span>
        <span>{taskAction.name}</span>
        <span>{$actions.get(taskAction.action_id)?.name ?? 'Unknown'}</span>
        <span
          ><Button iconButton on:click={() => editTaskAction(taskActionId)}><Pencil /></Button
          ></span
        >
      {/each}
    </div>
    <Modal bind:open={openTaskActionEditor} let:close let:data>
      <!-- <TaskActionEditor actionId={data.actionId} action={data.action} {close} /> -->
    </Modal>
  </Card>

  <Card class="mt-4 flex flex-col">
    <p class="section-header">Triggers</p>
    <div class="w-full task-item-list">
      <span class="font-medium">Trigger ID</span>
      <span class="font-medium">Trigger Name</span>
      <span class="font-medium">Input Type</span>
      <span />

      {#each Object.entries(task.triggers) as [taskTriggerId, trigger] (taskTriggerId)}
        <span>{taskTriggerId}</span>
        <span>{trigger.name}</span>
        <span>{$inputs.get(trigger.input_id)?.name ?? 'Unknown'}</span>
        <span
          ><Button iconButton on:click={() => editTaskTrigger(taskTriggerId)}><Pencil /></Button
          ></span
        >
      {/each}
    </div>
    <Modal bind:open={openTaskTriggerEditor} let:close let:data>
      <!-- TaskTriggerEditor triggerId={data.triggerId} trigger={data.trigger} {close} /> -->
    </Modal>
  </Card>

  <Card class="flex flex-col flex-grow mt-4 h-[64em]">
    {#if taskSource}
      <div class="flex-1 grid grid-rows-1 grid-cols-1 place-items-stretch">
        <svelte:component
          this={taskEditors[taskSource.type]}
          bind:getState={getEditorState}
          source={task.source?.data}
          compiled={task.compiled?.data}
          taskTriggers={task.triggers}
          taskActions={task.actions}
          {validator}
        />
      </div>
    {/if}
  </Card>
</div>

{#if newTask}
  <Modal bind:open={showNewTaskDialog} let:close>
    <p class="flex space-x-2">
      <Button on:click={() => close('StateMachine')}>State Machine</Button>
      <Button on:click={() => close('Js')}>Script</Button>
      <Button on:click={() => close('Flowchart')}>FlowChart</Button>
    </p>
  </Modal>
{/if}

<style lang="postcss">
  .section-header {
    @apply font-bold text-gray-700 dark:text-gray-300;
  }

  .task-item-list {
    display: grid;
    grid-template-columns: repeat(3, 1fr) auto;
    gap: 0.5rem 1rem;
  }
</style>
