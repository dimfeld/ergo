<script lang="ts">
  import { slide } from 'svelte/transition';
  import type { InputsLogEntry } from '../api_types';
  import InputsLogTimelineRow from './InputsLogTimelineRow.svelte';

  export let entries: InputsLogEntry[];
  let classNames = '';
  export { classNames as class };
</script>

<ul class="-mb-8 flex flex-col items-stretch {classNames}">
  {#each entries as entry (entry.inputs_log_id)}
    <li class="w-full" transition:slide|local={{ duration: 300 }}>
      <InputsLogTimelineRow {entry} />
    </li>
  {:else}
    <li class="font-medium text-gray-800 dark:text-gray-300">No recent log runs</li>
  {/each}
</ul>
