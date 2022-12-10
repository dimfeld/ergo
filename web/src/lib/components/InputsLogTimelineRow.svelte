<script lang="ts">
  import type { InputsLogEntry } from '../api_types';
  import ActionsLogTimelineRow from './ActionsLogTimelineRow.svelte';
  import { relativeTime } from '../time';

  export let entry: InputsLogEntry;

  $: failed = entry.input_status === 'error';
</script>

<div class="relative w-full pb-6">
  <div class="flex flex-col w-full space-y-3">
    <div class="relative flex w-full space-x-3">
      <div>
        <span
          class="h-6 w-6 mt-1 rounded-full bg-accent-300 dark:bg-accent-500 flex items-center justify-center ring-4 ring-white dark:ring-gray-600"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-4 w-4 text-white dark:text-gray-800"
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
        <div class="min-w-0 flex-1 pt-1.5 flex flex-col sm:flex-row justify-between sm:space-x-4">
          <div>
            <p class:failed class="title-row">
              <span class="bolded">{entry.task_name}</span>

              {failed ? 'failed to process input' : 'received input'}
              <span class="bolded">{entry.task_trigger_name}</span>
            </p>
          </div>
          <div class="text-left sm:text-right text-sm whitespace-nowrap text-gray-500">
            <time datetime={entry.timestamp}>{relativeTime(entry.timestamp)}</time> ago
          </div>
        </div>
      </div>
    </div>

    {#each entry.actions as action}
      <div class="relative ml-5">
        <ActionsLogTimelineRow parent={entry} entry={action} />
      </div>
    {/each}
  </div>
</div>

<style lang="postcss">
  .title-row {
    @apply text-sm text-gray-500;
    &.failed {
      @apply text-red-500;

      .bolded {
        @apply text-red-900;
      }
    }

    .bolded {
      @apply font-medium text-gray-900;
    }
  }

  :global(.dark) .title-row {
    &.failed .bolded {
      @apply text-red-300;
    }

    .bolded {
      @apply text-gray-300;
    }
  }
</style>
