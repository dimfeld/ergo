<script lang="ts">
  import Icon from '../Icon.svelte';

  let classNames = 'inline h-5 w-5';
  export { classNames as class };

  // heroicons.com plus
  const icon = `
    <path
    fill-rule="evenodd"
    d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z"
    clip-rule="evenodd"
    />
    `;
</script>

<Icon {icon} class={classNames} viewBox="0 0 20 20" />
