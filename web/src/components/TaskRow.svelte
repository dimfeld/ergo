<script lang="ts">
  import type { TaskDescription } from '../api_types';
  import { formatISO9075, formatDistanceToNowStrict } from 'date-fns';
  import Badge from './Badge.svelte';

  export let task: TaskDescription;
</script>

<a href="/tasks/{task.id}" class="block hover:bg-accent-50 dark:hover:bg-gray-800">
  <div class="px-4 py-4 sm:px-6">
    <div class="flex items-center justify-between">
      <p class="text-sm font-medium text-accent-600 dark:text-accent-400 truncate">{task.name}</p>
      <div class="ml-2 flex-shrink-0 flex items-center space-x-2">
        <span class="text-xs text-gray-500 dark:text-gray-400">Runs in last week</span>
        <Badge style="success">{task.successes} successes</Badge>
        {#if task.failures}
          <Badge style="error">{task.failures} errors</Badge>
        {/if}
      </div>
    </div>
    <div class="mt-2 sm:flex sm:justify-between">
      <div class="sm:flex">
        <p class="flex items-center text-sm text-gray-500 dark:text-gray-400">
          ID: {task.id}
        </p>
        <p class="mt-2 flex items-center text-sm text-gray-500 dark:text-gray-400 sm:mt-0 sm:ml-6">
          Updated {formatISO9075(new Date(task.modified), { representation: 'date' })}
        </p>
      </div>
      <div class="mt-2 flex items-center text-sm text-gray-500 dark:text-gray-400 sm:mt-0">
        <!-- Heroicon name: solid/calendar -->
        <svg
          class="flex-shrink-0 mr-1.5 h-5 w-5 text-gray-400 dark:text-gray-500"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 20 20"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            fill-rule="evenodd"
            d="M6 2a1 1 0 00-1 1v1H4a2 2 0 00-2 2v10a2 2 0 002 2h12a2 2 0 002-2V6a2 2 0 00-2-2h-1V3a1 1 0 10-2 0v1H7V3a1 1 0 00-1-1zm0 5a1 1 0 000 2h8a1 1 0 100-2H6z"
            clip-rule="evenodd"
          />
        </svg>
        <p>
          Last run
          <time datetime={task.last_triggered}
            >{formatDistanceToNowStrict(new Date(task.last_triggered))}</time
          > ago
        </p>
      </div>
    </div>
  </div>
</a>
