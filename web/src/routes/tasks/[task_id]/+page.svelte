<script lang="ts">
  import { goto, invalidate } from '$app/navigation';
  import { page } from '$app/stores';
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import Modal, { type ModalOpener } from '$lib/components/Modal.svelte';
  import type { TaskAction, TaskResult, TaskTrigger } from '$lib/api_types';
  import { getHeaderTextStore } from '$lib/header';
  import { onDestroy } from 'svelte';
  import clone from 'just-clone';

  import ScriptEditor from '$lib/editors/Script.svelte';
  import StateMachineEditor from '$lib/editors/StateMachine.svelte';
  import DataFlowEditor from '$lib/editors/DataFlow.svelte';
  import { baseData } from '$lib/data';
  import apiClient from '$lib/api';
  import { new_task_id, new_task_trigger_id, TaskConfigValidator } from 'ergo-wasm';
  import initWasm from '$lib/wasm';
  import Labelled from '$lib/components/Labelled.svelte';
  import Pencil from '$lib/components/icons/Pencil.svelte';
  import TaskTriggerEditor, { type TaskTriggerEditorData } from '../_TaskTriggerEditor.svelte';
  import TaskActionEditor, { type TaskActionEditorData } from '../_TaskActionEditor.svelte';
  import type { PageData } from './$types';

  export let data: PageData;
  let task: TaskResult;
  $: task = task ?? data.task ?? defaultTask();

  const taskEditors = {
    Js: { component: ScriptEditor, padding: true },
    StateMachine: { component: StateMachineEditor, padding: true },
    DataFlow: { component: DataFlowEditor, padding: false, focusBorder: true },
  };

  const { inputs, actions } = baseData();
  const client = apiClient();

  function taskId() {
    return task.task_id || new_task_id();
  }

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
    let taskType = task.source?.type ?? task.compiled?.type;
    if (!taskType) {
      return;
    }

    let { source, compiled } = await getEditorState();
    task.source = source;
    task.compiled = compiled;

    if (newTask) {
      let result = await client.post(`/api/tasks`, { json: task }).json<{ task_id: string }>();

      // Update all the tasks IDs with the new one.
      task.task_id = result.task_id;
      for (let trigger of Object.values(task.triggers)) {
        trigger.task_id = result.task_id;
      }

      for (let action of Object.values(task.actions)) {
        action.task_id = result.task_id;
      }

      goto(result.task_id, { replaceState: true, noScroll: true, keepFocus: true });
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

  function newTaskAction(): TaskAction {
    return {
      name: '',
      task_id: taskId(),
      action_id: $actions.keys().next().value,
      task_local_id: '',
      account_id: null,
      action_template: null,
    };
  }

  let openTaskActionEditor: ModalOpener<Partial<TaskActionEditorData>, TaskActionEditorData>;
  async function editTaskAction(taskActionId: string | undefined) {
    let result = await openTaskActionEditor({
      taskActionId,
      action: taskActionId ? task.actions[taskActionId] : newTaskAction(),
    });
    if (result) {
      if (taskActionId && result.taskActionId !== taskActionId) {
        delete task.actions[taskActionId];
      }

      task.actions[result.taskActionId] = result.action;
    }
  }

  function newTaskTrigger(): TaskTrigger {
    return {
      task_id: taskId(),
      name: '',
      input_id: $inputs.keys().next().value,
      task_trigger_id: new_task_trigger_id(),
      description: null,
      periodic: null,
    };
  }

  let openTaskTriggerEditor: ModalOpener<Partial<TaskTriggerEditorData>, TaskTriggerEditorData>;
  async function editTaskTrigger(triggerId: string | undefined) {
    let result = await openTaskTriggerEditor({
      triggerId,
      trigger: triggerId ? clone(task.triggers[triggerId]) : newTaskTrigger(),
    });
    if (result) {
      if (triggerId && result.triggerId !== triggerId) {
        delete task.triggers[triggerId];
      }

      task.triggers[result.triggerId] = result.trigger;
    }
  }
</script>

<div class="flex flex-grow flex-col">
  <section class="flex flex-row justify-end space-x-4">
    <!-- TODO add confirmation dropdown -->
    <Button on:click={revert}>Revert</Button>
    <Button style="primary" on:click={save}>Save</Button>
  </section>
  <Card class="mt-2 flex flex-col space-y-4">
    <div class="flex w-full justify-between space-x-4">
      <Labelled label="Name" class="w-full"
        ><input class="w-full" type="text" bind:value={task.name} /></Labelled>
      <Labelled label="Alias">
        <input type="text" bind:value={task.alias} placeholder="None" />
      </Labelled>
    </div>
    <Labelled label="Description"
      ><input type="text" class="w-full" bind:value={task.description} /></Labelled>
    <div class="flex justify-between space-x-4">
      <p class="whitespace-nowrap text-sm">
        ID: <span class:text-gray-500={!task.task_id}>{task.task_id || 'New Task'}</span>
      </p>
      <p class="text-sm">Modified {task.modified}</p>
    </div>
  </Card>

  <Card class="mt-4 flex flex-col" label="Actions">
    <div class="task-item-list w-full">
      <span class="font-medium">Local ID</span>
      <span class="font-medium">Description</span>
      <span class="font-medium">Action Type</span>
      <span />
      {#each Object.entries(task.actions) as [taskActionId, taskAction] (taskActionId)}
        <span>{taskActionId}</span>
        <span>{taskAction.name}</span>
        <span>{$actions.get(taskAction.action_id)?.name ?? 'Unknown'}</span>
        <span>
          <Button iconButton on:click={() => editTaskAction(taskActionId)}><Pencil /></Button>
        </span>
      {/each}
    </div>

    <div class="mt-2 items-start">
      <Button on:click={() => editTaskAction(undefined)}>New Task Action</Button>
    </div>

    <Modal bind:open={openTaskActionEditor} let:close let:data>
      <TaskActionEditor
        allActions={task.actions}
        taskActionId={data.taskActionId}
        action={data.action}
        {close} />
    </Modal>
  </Card>

  <Card class="mt-4 flex flex-col" label="Triggers">
    <div class="task-item-list w-full">
      <span class="font-medium">Trigger ID</span>
      <span class="font-medium">Description</span>
      <span class="font-medium">Input Type</span>
      <span />

      {#each Object.entries(task.triggers) as [taskTriggerId, trigger] (taskTriggerId)}
        <span>{taskTriggerId}</span>
        <span>{trigger.name}</span>
        <span>{$inputs.get(trigger.input_id)?.name ?? 'Unknown'}</span>
        <span>
          <Button iconButton on:click={() => editTaskTrigger(taskTriggerId)}><Pencil /></Button>
        </span>
      {/each}
    </div>

    <div class="mt-2 items-start">
      <Button on:click={() => editTaskTrigger(null)}>New Task Trigger</Button>
    </div>

    <Modal bind:open={openTaskTriggerEditor} let:close let:data>
      <TaskTriggerEditor
        allTriggers={task.triggers}
        triggerId={data.triggerId}
        trigger={data.trigger}
        {close} />
    </Modal>
  </Card>

  {#if taskSource}
    {@const editor = taskEditors[taskSource.type]}
    <Card
      class="mt-4 flex min-h-[64em] flex-grow flex-col overflow-hidden"
      padding={editor.padding ?? true}>
      <div class="grid min-h-0 flex-1 grid-cols-1 grid-rows-1 place-items-stretch">
        <svelte:component
          this={editor.component}
          bind:getState={getEditorState}
          source={task.source?.data}
          compiled={task.compiled?.data}
          taskTriggers={task.triggers}
          taskActions={task.actions}
          {validator} />
      </div>
    </Card>
  {/if}
</div>

{#if newTask}
  <Modal bind:open={showNewTaskDialog} let:close>
    <p class="flex space-x-2">
      <Button on:click={() => close('StateMachine')}>State Machine</Button>
      <Button on:click={() => close('Js')}>Script</Button>
      <Button on:click={() => close('DataFlow')}>Data Flow</Button>
    </p>
  </Modal>
{/if}

<style lang="postcss">
  .task-item-list {
    display: grid;
    grid-template-columns: repeat(3, 1fr) auto;
    gap: 0.5rem 1rem;
  }
</style>
