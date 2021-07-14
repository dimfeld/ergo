<script lang="ts">
  import type { InputsLogEntry } from '../api_types';
  import InputsLogTimelineRow from './InputsLogTimelineRow.svelte';

  export let entries: InputsLogEntry[];
</script>

<ul class="-mb-8 flex flex-col items-stretch">
  {#each entries as entry}
    <li class="w-full">
      <InputsLogTimelineRow {entry} />
    </li>
  {/each}
</ul>
