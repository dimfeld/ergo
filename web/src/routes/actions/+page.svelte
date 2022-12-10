<script lang="ts">
  import { baseData } from '$lib/data';
  import { getHeaderTextStore } from '$lib/header';
  import Card from '$lib/components/Card.svelte';
  import Labelled from '$lib/components/Labelled.svelte';
  import ActionEditor from '../_ActionEditor.svelte';
  import Modal, { type ModalOpener } from '$lib/components/Modal.svelte';
  import type { Action } from '$lib/api_types';
  import clone from 'just-clone';
  import apiClient from '$lib/api';
  import { goto, invalidate } from '$app/navigation';
  import Button from '$lib/components/Button.svelte';
  const { actions } = baseData();

  getHeaderTextStore().set(['Actions']);

  let openDialog: ModalOpener<Action, Action>;
  async function editAction(action: Action | undefined) {
    goto(`/actions/${action?.action_id ?? 'new'}`);
  }
</script>

<a href="/actions/new"><Button>New Action</Button></a>

<ul class="mt-4 space-y-4">
  {#each Array.from($actions.values()) as action (action.action_id)}
    <li>
      <Card class="flex">
        <div>
          <p>
            <span class="font-medium text-gray-800 dark:text-gray-200">{action.name}</span>
            {#if action.description} &mdash; {action.description}{/if}
          </p>
          <div class="ml-4">
            <p />
            <p><span class="font-medium">Executor:</span> {action.executor_id}</p>
            {#if action.timeout}
              <p>Timeout: {action.timeout} seconds</p>
            {/if}
            <div>
              <Labelled label="Action Inputs">
                <ul class="ml-4">
                  {#each action.template_fields as templateField}
                    <li>
                      {templateField.name} &mdash;
                      {JSON.stringify(templateField.format)}
                    </li>
                  {/each}
                </ul>
              </Labelled>
            </div>
            <div>
              <Labelled label="Executor Template">
                {#if action.executor_template.t === 'Template'}
                  <ul class="ml-4">
                    {#each action.executor_template.c as [field, value] (field)}
                      <li>{field} &mdash; {JSON.stringify(value)}</li>
                    {/each}
                  </ul>
                {:else if action.executor_template.t === 'Script'}
                  <code><pre>{action.executor_template.c}</pre></code>
                {/if}
              </Labelled>
            </div>
            {#if action.account_types?.length}
              <p>
                Account Types{#if action.account_required}
                  (required){/if}:{action.account_types.join(', ')}
              </p>
            {/if}
            {#if action.postprocess_script}
              <Labelled label="Postprocessing Script">
                <code><pre class="whitespace-pre-wrap">{action.postprocess_script}</pre></code>
              </Labelled>
            {/if}
          </div>
        </div>
        <Button class="ml-auto self-start" on:click={() => editAction(action)}>Edit</Button>
      </Card>
    </li>
  {/each}
</ul>

<Modal bind:open={openDialog} let:data let:close>
  <ActionEditor {close} {data} />
</Modal>
