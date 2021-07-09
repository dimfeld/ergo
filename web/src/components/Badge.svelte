<script lang="ts">
  export let style: 'success' | 'error' | 'plain' | 'accent' = 'plain';

  const styles = {
    success: 'bg-green-100 text-green-800 dark:bg-green-800 dark:text-green-200',
    error: 'bg-red-100 text-red-800 dark:bg-red-800 dark:text-red-200',
    plain: 'bg-gray-200 text-gray-800 dark:bg-gray-700 dark:text-gray-200',
    accent: 'bg-primary-100 text-primary-800 dark:bg-primary-800 dark:text-primary-200',
  };
</script>

<p class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full {styles[style]}">
  <slot />
</p>
