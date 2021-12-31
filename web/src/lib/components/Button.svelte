<script lang="ts">
  export let element: HTMLButtonElement | undefined = undefined;
  export let disabled = false;

  export let type: HTMLButtonElement['type'] = 'button';
  export let size: keyof typeof sizes = 'md';
  export let style: keyof typeof styles = 'normal';
  export let iconButton = false;
  export let title: string | undefined = undefined;

  let classNames = '';
  export { classNames as class };

  const sizes = {
    xs: 'text-xs rounded',
    sm: 'leading-4 text-sm rounded-md',
    md: 'text-sm rounded-md',
  };

  const sizePadding = {
    xs: 'px-2.5 py-1.5',
    sm: 'px-3 py-2',
    md: 'px-4 py-2',
  };

  const iconSizePadding = {
    xs: 'p-1',
    sm: 'p-1.5',
    md: 'p-1',
  };

  const styles = {
    normal:
      'border-gray-300 dark:border-gray-700 bg-white dark:bg-black hover:bg-accent-50 dark:hover:border-gray-600 dark:hover:bg-gray-800 focus:ring-accent-500 dark:focus:ring-accent-700 ',
    primary:
      'border-accent-300 dark:border-accent-600 bg-accent-100 dark:bg-black hover:bg-accent-200 dark:hover:bg-gray dark:hover:border-accent-500 dark:hover:bg-accent-900 dark:hover:bg-opacity-50 focus:ring-accent-500 dark:focus:ring-accent-700 ',
    danger:
      'border-red-300 dark:border-red-700 bg-white dark:bg-black hover:bg-red-50 dark:hover:bg-red-900 focus:ring-red-500 dark:focus:ring-accent-700',
  };

  $: dynamicClasses = `${sizes[size]} ${iconButton ? iconSizePadding[size] : sizePadding[size]} ${
    styles[style]
  } ${classNames}`;
</script>

<button
  bind:this={element}
  {type}
  {disabled}
  {title}
  on:click
  class="inline-flex justify-center items-center border shadow-sm font-medium text-gray-700 disabled:text-gray-400 dark:text-gray-300 dark:disabled:text-gray-400 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-100 dark:focus:ring-offset-gray-900 transition-colors duration-200 {dynamicClasses}"
  class:cursor-not-allowed={disabled}
  ><slot />
</button>
