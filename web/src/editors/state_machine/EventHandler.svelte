<script lang="ts">
  import { EventHandler } from '$lib/api_types';
  export let handler: EventHandler;
</script>

<p>
  On trigger <span class="font-medium text-accent-800 dark:text-accent-200"
    >{handler.trigger_id}</span
  >
</p>
{#each handler.actions ?? [] as action}
  <div class="ml-4">
    <p>
      Run action <span class="font-medium text-accent-800 dark:text-accent-200"
        >{action.task_action_local_id}</span
      >
    </p>
  </div>
{/each}
