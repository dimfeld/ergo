<script lang="ts">
  import Icon from '../Icon.svelte';

  // heroicons.com chevron-down
  let classNames = 'inline h-6 w-6';
  export { classNames as class };

  const icon = `
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
  `;
</script>

<Icon {icon} class={classNames} fill="none" viewBox="0 0 24 24" stroke="currentColor" />
