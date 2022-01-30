<script context="module" lang="ts">
  import { Action } from '$lib/api_types';
  import clone from 'just-clone';
  import type { Load } from '@sveltejs/kit';
  import pascalCase from 'just-pascal-case';
  import { new_action_id } from 'ergo-wasm';

  function newAction(): Action {
    return {
      name: '',
      executor_id: '',
      template_fields: [],
      executor_template: { t: 'Template', c: [] },
      account_required: false,
      action_category_id: undefined, // TODO
    };
  }

  export const load: Load = async function load({ stuff, params }) {
    let { action_id } = params;

    let action = action_id !== 'new' ? stuff.actions.get(action_id) : newAction();
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
  import apiClient from '$lib/api';
  import { getHeaderTextStore } from '$lib/header';
  import { goto, invalidate } from '$app/navigation';
  import Card from '$lib/components/Card.svelte';
  import Editor from '$lib/editors/Editor.svelte';
  import AnyEditor from '$lib/components/AnyEditor.svelte';

  export let action: Action;

  const api = apiClient();
  const { executors } = baseData();

  $: actionName = action.name ?? (action.action_id ? '' : 'New Action');

  const header = getHeaderTextStore();
  $: $header = ['Actions', actionName];

  $: executor = $executors.get(action.executor_id);

  let postprocessContents: () => string;
  let actionCategories = {}; // TODO

  $: executorTemplateArguments =
    action.executor_template.t === 'Template'
      ? Object.fromEntries(
          action.executor_template.c.map(([name, value], index) => {
            return [
              name,
              {
                value,
                index,
              },
            ];
          })
        )
      : {};

  function updateExecutorTemplateValue(name: string, value: any) {
    if (action.executor_template.t === 'Template') {
      let templateValueIndex = action.executor_template.c.findIndex((v) => v[0] === name);
      if (templateValueIndex >= 0) {
        if (value === null) {
          // Remove the item from the template
          action.executor_template.c = [
            ...action.executor_template.c.slice(0, templateValueIndex),
            ...action.executor_template.c.slice(templateValueIndex + 1),
          ];
        } else {
          action.executor_template.c[templateValueIndex][1] = value;
        }
      } else {
        action.executor_template.c = [...action.executor_template.c, [name, value]];
      }
    }
  }

  function changeExecutor(e: InputEvent) {
    action.executor_id = e.target.value;
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
          {#each Object.entries(actionCategories) as [id, name]}
            <option value={id}>{name}</option>
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
  <Card class="flex flex-col space-y-4" label="Template Inputs">
    <ul class="flex flex-col space-y-4">
      {#each action.template_fields as template_field (template_field.name)}
        <li>
          <pre>{JSON.stringify(template_field)}</pre>
        </li>
      {/each}
    </ul>
  </Card>
  <Card label="Executor Template">
    <!-- TODO script/template toggle -->
    <ul class="flex flex-col space-y-4">
      {#each executor?.template_fields || [] as field, i}
        <li>
          <Labelled
            label={field.name}
            help="{pascalCase(field.format.type)} &mdash; {field.description}"
          >
            <AnyEditor
              type={field.format.type}
              value={executorTemplateArguments[field.name]?.value}
              on:change={(e) => updateExecutorTemplateValue(field.name, e.detail)}
            />
          </Labelled>
        </li>
      {/each}
    </ul>
  </Card>
  <Card label="Accounts">
    <Checkbox bind:value={action.account_required} label="Account Required?" />
    <!-- account types -->
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
