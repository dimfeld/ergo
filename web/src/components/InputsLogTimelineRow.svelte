<script lang="ts">
  import type { InputsLogEntry } from '../api_types';
  import ActionsLogTimelineRow from './ActionsLogTimelineRow.svelte';
  import { relativeTime } from '../time';

  export let entry: InputsLogEntry;
</script>

<div class="relative pb-8 w-full">
  <span
    class="absolute top-4 left-4 -ml-px h-full w-0.5 bg-gray-200 dark:bg-gray-800"
    aria-hidden="true"
  />
  <div class="relative flex w-full space-x-3">
    <div>
      <span
        class="h-8 w-8 rounded-full bg-accent-300 dark:bg-accent-500 flex items-center justify-center ring-8 ring-white dark:ring-gray-600"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="h-5 w-5 text-white dark:text-gray-800"
          viewBox="0 0 20 20"
          fill="currentColor"
        >
          <path
            fill-rule="evenodd"
            d="M5 2a2 2 0 00-2 2v14l3.5-2 3.5 2 3.5-2 3.5 2V4a2 2 0 00-2-2H5zm4.707 3.707a1 1 0 00-1.414-1.414l-3 3a1 1 0 000 1.414l3 3a1 1 0 001.414-1.414L8.414 9H10a3 3 0 013 3v1a1 1 0 102 0v-1a5 5 0 00-5-5H8.414l1.293-1.293z"
            clip-rule="evenodd"
          />
        </svg>
      </span>
    </div>
    <div class="flex flex-col space-y-8 flex-grow">
      <div class="min-w-0 flex-1 pt-1.5 flex justify-between space-x-4">
        <div>
          <p class="text-sm text-gray-500">
            <span class="font-medium text-gray-900 dark:text-gray-300">{entry.task_name}</span>
            received input
            <span class="font-medium text-gray-900 dark:text-gray-300"
              >{entry.task_trigger_name}</span
            >
          </p>
        </div>
        <div class="text-right text-sm whitespace-nowrap text-gray-500">
          <time datetime={entry.timestamp}>{relativeTime(entry.timestamp)}</time> ago
        </div>
      </div>
      {#each entry.actions as action}
        <div class="relative">
          <span
            class="absolute -left-7 top-4 w-8 h-0.5 bg-gray-200 dark:bg-gray-800"
            aria-hidden="true"
          />
          <ActionsLogTimelineRow parent={entry} entry={action} />
        </div>
      {/each}
    </div>
  </div>
</div>
