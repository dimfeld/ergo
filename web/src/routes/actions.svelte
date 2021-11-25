<script lang="ts">
  import { baseData } from '$lib/data';
  import { getHeaderTextStore } from '$lib/header';
  import Card from '$lib/components/Card.svelte';
  const { actions } = baseData();
  getHeaderTextStore().set(['Actions']);
</script>

<ul class="space-y-4">
  {#each Array.from($actions.values()) as action (action.action_id)}
    <li>
      <Card>
        <p>
          <span class="font-medium text-gray-800 dark:text-gray-200">{action.name}</span>
          {#if action.description} &mdash; {action.description}{/if}
        </p>
        <div class="ml-4">
          <p />
          <p>{action.executor_id}</p>
          <div>
            <p>Action Inputs</p>
            <ul class="ml-4">
              {#each action.template_fields as templateField}
                <li>
                  {templateField.name} &mdash;
                  {JSON.stringify(templateField.format)}
                </li>
              {/each}
            </ul>
          </div>
          <div>
            <p>Executor Template</p>
            {#if action.executor_template.t === 'Template'}
              <ul class="ml-4">
                {#each action.executor_template.c as [field, value] (field)}
                  <li>{field} &mdash; {JSON.stringify(value)}</li>
                {/each}
              </ul>
            {:else if action.executor_template.t === 'Script'}
              <code><pre>{action.executor_template.c}</pre></code>
            {/if}
          </div>
          {#if action.account_types?.length}
            <p>
              Account Types{#if action.account_required} (required){/if}:{action.account_types.join(
                ', '
              )}
            </p>
          {/if}
          {#if action.timeout}
            <p>Timeout: {action.timeout} seconds</p>
          {/if}
          {#if action.postprocess_script}
            <p>Postprocessing Script</p>
            <code><pre class="whitespace-pre-wrap">{action.postprocess_script}</pre></code>
          {/if}
        </div>
      </Card>
    </li>
  {/each}
</ul>
