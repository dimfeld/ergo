<script lang="ts">
  import Icon from '../Icon.svelte';

  let classNames = 'inline h-5 w-5';
  export { classNames as class };

  // heroicons.com pencil
  const icon = `
  <path
    d="M13.586 3.586a2 2 0 112.828 2.828l-.793.793-2.828-2.828.793-.793zM11.379 5.793L3 14.172V17h2.828l8.38-8.379-2.83-2.828z"
  />

  `;
</script>

<Icon {icon} class={classNames} fill="currentColor" viewBox="0 0 20 20" stroke="currentColor" />
