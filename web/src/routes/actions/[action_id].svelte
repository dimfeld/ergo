<script context="module" lang="ts">
  import { Action, ActionCategory, TemplateFieldFormat } from '$lib/api_types';
  import clone from 'just-clone';
  import type { Load } from '@sveltejs/kit';
  import * as help from './_helpText';
  import { new_action_id } from 'ergo-wasm';

  function newAction(actionCategories: Map<string, ActionCategory>): Action {
    return {
      name: '',
      executor_id: '',
      template_fields: [],
      executor_template: { t: 'Template', c: [] },
      account_required: false,
      action_category_id: actionCategories.keys().next().value,
    };
  }

  export const load: Load = async function load({ stuff, params }) {
    let { action_id } = params;

    let action =
      action_id !== 'new' ? stuff.actions.get(action_id) : newAction(stuff.actionCategories);
    if (!action) {
      return {
        status: 404,
        error: 'Action not found',
      };
    }

    return {
      props: {
        action: clone(action),
      },
    };
  };
</script>

<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Labelled from '$lib/components/Labelled.svelte';
  import { baseData } from '$lib/data';
  import { page } from '$app/stores';
  import apiClient from '$lib/api';
  import { getHeaderTextStore } from '$lib/header';
  import { goto, invalidate } from '$app/navigation';
  import Card from '$lib/components/Card.svelte';
  import Editor from '$lib/editors/Editor.svelte';
  import TemplateFieldsEditor from '$lib/components/TemplateFieldsEditor.svelte';
  import TemplateValuesEditor from '$lib/components/TemplateValuesEditor.svelte';
  import StringListEditor from '$lib/components/StringListEditor.svelte';

  export let action: Action;

  const api = apiClient();
  const { accountTypes, actionCategories, executors } = baseData();

  $: actionName = $page.params.action_id === 'new' ? 'New Action' : action.name;

  const header = getHeaderTextStore();
  $: $header = ['Actions', actionName];

  $: executor = $executors.get(action.executor_id);

  let postprocessContents: () => string;

  function changeExecutor(e: InputEvent) {
    action.executor_id = e.target.value;
    // None of the old template values will apply to the new template, so clear it out.
    action.executor_template = {
      t: 'Template',
      c: [],
    };
  }

  async function handleSubmit() {
    if (!action.name) {
      // TODO error message
      return;
    }

    action.postprocess_script = postprocessContents();

    if (action.action_id) {
      await api.put(`/api/actions/${action.action_id}`, {
        json: action,
      });
    } else {
      let result = await api
        .post(`/api/actions`, {
          json: action,
        })
        .json<Action>();
      goto(`/actions/${result.action_id}`, { replaceState: true, noscroll: true, keepfocus: true });
    }

    invalidate(`/api/actions`);
  }

  function wrapPostprocessCode() {
    return {
      prefix: 'function(output: any) {',
      suffix: '}',
    };
  }
</script>

<form on:submit|preventDefault={handleSubmit} class="flex flex-col space-y-6">
  <Card class="flex flex-col space-y-4">
    <div class="flex items-end space-x-4">
      <Labelled class="flex-1" label="Name"
        ><input type="text" class="w-full" bind:value={action.name} /></Labelled
      >
      <Labelled class="flex-1" label="Category">
        <select class="w-full " bind:value={action.action_category_id}>
          {#each Array.from($actionCategories.entries()) as [id, category]}
            <option value={id}>{category.name}</option>
          {/each}
        </select>
      </Labelled>
      <Button class="flex-none" style="primary" type="submit">Save</Button>
    </div>
    <Labelled label="Description"
      ><input type="text" class="w-full" bind:value={action.description} /></Labelled
    >
    <div class="flex space-x-4">
      <Labelled class="w-1/2" label="Executor">
        <select class="w-full" value={action.executor_id} on:change={changeExecutor}>
          {#each Array.from($executors.values()) as info}
            <option>{info.name}</option>
          {/each}
        </select>
      </Labelled>
      <Labelled class="w-1/2" label="Timeout (seconds)">
        <input
          class="w-full"
          type="number"
          bind:value={action.timeout}
          placeholder="Timeout in Seconds"
        />
      </Labelled>
    </div>
  </Card>
  <Card class="flex flex-col" label="Template Inputs" help={help.templateInputs}>
    <TemplateFieldsEditor bind:fields={action.template_fields} />
  </Card>
  <Card label="Executor Template" help={help.executorTemplate}>
    <TemplateValuesEditor
      fields={executor?.template_fields || []}
      bind:values={action.executor_template}
    />
  </Card>
  <Card class="flex flex-col" label="Account Types">
    <Checkbox bind:value={action.account_required} label="Account Required?" />
    <Labelled class="mt-4" label="Allowed Account Types">
      <StringListEditor
        bind:values={action.account_types}
        possible={Object.fromEntries(
          Array.from($accountTypes.values(), (a) => [a.account_type_id, a.name])
        )}
      />
    </Labelled>
  </Card>
  <Card label="Postprocessing" class="relative">
    <!-- postprocess script -->
    <div class="mt-2 flex max-h-[32rem] min-h-[12rem] w-full flex-col">
      <Editor
        format="js"
        wrapCode={wrapPostprocessCode}
        bind:getContents={postprocessContents}
        contents={action.postprocess_script || ''}
      />
    </div>
  </Card>
</form>
