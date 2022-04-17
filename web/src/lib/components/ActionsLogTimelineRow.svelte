<script lang="ts">
  import type { InputsLogEntry, InputLogEntryAction } from '../api_types';
  import { relativeTime } from '../time';

  export let parent: InputsLogEntry;
  export let entry: InputLogEntryAction;

  interface ResultDescription {
    action: string;
    item: string | null;
  }

  let resultDescription: ResultDescription | undefined;
  $: {
    let description = entry?.result?.output?.description;
    if (typeof description === 'string') {
      resultDescription = {
        action: description,
        item: null,
      };
    } else if (description && 'action' in description) {
      resultDescription = description;
    }
  }

  const statusMessages = {
    error: 'failed to run',
    running: 'is running',
    pending: 'is waiting to run',
    success: 'ran action',
  };
  $: statusVerb = statusMessages[entry.status] ?? statusMessages.success;
  $: failed = entry.status === 'error';
</script>

<div class="relative">
  <div class="relative flex items-center space-x-3">
    <div>
      <span
        class="mt-1 flex h-6 w-6 items-center justify-center rounded-full bg-accent-300 ring-4 ring-white dark:bg-accent-500 dark:ring-gray-600"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="h-4 w-4 text-white dark:text-gray-800"
          viewBox="0 0 20 20"
          fill="currentColor"
        >
          <path
            d="M8 5a1 1 0 100 2h5.586l-1.293 1.293a1 1 0 001.414 1.414l3-3a1 1 0 000-1.414l-3-3a1 1 0 10-1.414 1.414L13.586 5H8zM12 15a1 1 0 100-2H6.414l1.293-1.293a1 1 0 10-1.414-1.414l-3 3a1 1 0 000 1.414l3 3a1 1 0 001.414-1.414L6.414 15H12z"
          />
        </svg>
      </span>
    </div>
    <div class="flex min-w-0 flex-1 flex-col justify-between pt-1.5 sm:flex-row sm:space-x-4">
      <div>
        <p class:failed class="title-row">
          {#if failed || !resultDescription}
            <span class="bolded">{parent.task_trigger_name}</span>
            {statusVerb}
            <span class="bolded">{entry.task_action_name}</span>
          {:else}
            <span>{resultDescription.action}</span>
            {#if resultDescription.item}
              <span class="bolded">{resultDescription.item}</span>
            {/if}
          {/if}
        </p>
      </div>
      <div class="whitespace-nowrap text-left text-sm text-gray-500 sm:text-right">
        <time datetime={entry.timestamp}>{relativeTime(entry.timestamp)}</time> ago
      </div>
    </div>
  </div>
</div>

<style lang="postcss">
  .title-row {
    @apply text-sm text-gray-600;
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
    @apply text-gray-400;

    &.failed .bolded {
      @apply text-red-300;
    }

    .bolded {
      @apply text-gray-300;
    }
  }
</style>
