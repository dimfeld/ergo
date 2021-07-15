<script lang="ts">
  import type { InputsLogEntry, InputLogEntryAction } from '../api_types';
  import { relativeTime } from '../time';

  export let parent: InputsLogEntry;
  export let entry: InputLogEntryAction;
  $: failed = entry.status === 'error';
</script>

<div class="relative">
  <div class="relative flex space-x-3">
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
            d="M8 5a1 1 0 100 2h5.586l-1.293 1.293a1 1 0 001.414 1.414l3-3a1 1 0 000-1.414l-3-3a1 1 0 10-1.414 1.414L13.586 5H8zM12 15a1 1 0 100-2H6.414l1.293-1.293a1 1 0 10-1.414-1.414l-3 3a1 1 0 000 1.414l3 3a1 1 0 001.414-1.414L6.414 15H12z"
          />
        </svg>
      </span>
    </div>
    <div class="min-w-0 flex-1 pt-1.5 flex justify-between space-x-4">
      <div>
        <p class:failed class="title-row">
          <span class="bolded">{parent.task_trigger_name}</span>
          {failed ? 'failed to run' : 'ran action'}
          <span class="bolded">{entry.task_action_name}</span>
        </p>
      </div>
      <div class="text-right text-sm whitespace-nowrap text-gray-500">
        <time datetime={entry.timestamp}>{relativeTime(entry.timestamp)}</time> ago
      </div>
    </div>
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
