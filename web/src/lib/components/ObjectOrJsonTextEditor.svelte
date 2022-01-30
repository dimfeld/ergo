<script lang="ts">
  import ObjectEditor from './ObjectEditor.svelte';
  import Editor from '$lib/editors/Editor.svelte';
  import Switch from './Switch.svelte';
  import { formatJson } from '$lib/editors/format';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  export let value: object;
  export let complexMode = true;

  $: console.log({ value });

  $: useEditor = complexMode || simpleModeDisallowed;
  $: simpleModeDisallowed = Object.values(value).some((v) => typeof v === 'object');

  function updateText(text: string) {
    try {
      value = JSON.parse(text);
      dispatch('change', value);
    } catch (e) {}
  }
</script>

<div class="flex w-full flex-col items-stretch space-y-2">
  <div class="flex justify-end space-x-2">
    <Switch name="complex_mode" bind:value={complexMode} disabled={simpleModeDisallowed}
      ><span class="text-sm font-medium">Edit as Text</span></Switch
    >
  </div>

  {#if useEditor}
    <Editor
      format="json"
      toolbar={false}
      notifyOnChange={true}
      contents={formatJson(JSON.stringify(value), 'json')}
      on:change={({ detail: text }) => updateText(text)}
    />
  {:else}
    <ObjectEditor bind:value on:change />
  {/if}
</div>
