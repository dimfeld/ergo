<script lang="ts">
  export let element: HTMLButtonElement | undefined = undefined;
  export let disabled = false;

  export let size: keyof typeof sizes = 'md';
  export let style: keyof typeof styles = 'normal';

  let classNames = '';
  export { classNames as class };

  const sizes = {
    xs: 'px-2.5 py-1.5 text-xs rounded',
    sm: 'px-3 py-2 leading-4 text-sm rounded-md',
    md: 'px-4 py-2 text-sm rounded-md',
  };

  const styles = {
    primary: 'border-accent-300 dark:border-accent-700',
    normal: 'border-gray-300 dark:borde-gray-700',
  };

  $: dynamicClasses = `${sizes[size]} ${styles[style]} ${classNames}`;
</script>

<button
  bind:this={element}
  type="button"
  {disabled}
  on:click
  class="inline-flex justify-center border shadow-sm bg-white dark:bg-black text-sm font-medium text-gray-700 disabled:text-gray-400 dark:text-gray-300 dark:disabled:text-gray-400 hover:bg-accent-50 dark:hover:bg-gray-900 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-100 dark:focus:ring-offset-gray-900 focus:ring-accent-500 dark:focus:ring-accent-700 {dynamicClasses}"
  class:cursor-not-allowed={disabled}
  ><slot />
</button>
