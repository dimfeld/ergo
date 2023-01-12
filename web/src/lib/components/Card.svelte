<script lang="ts">
  import { cls } from '$lib/styles';
  import HelpButton from './HelpButton.svelte';

  let classNames: string | undefined = undefined;
  export { classNames as class };

  export let label = '';
  export let help = '';
  export let padding = true;
</script>

<section
  class={cls(
    'w-full rounded border border-gray-200 shadow-md dark:border-gray-400',
    padding && 'p-4',
    classNames
  )}>
  {#if label}
    <header class="mb-4 flex font-bold text-gray-700 dark:text-gray-300">
      <span>{label}</span>
      {#if help}
        <div class="ml-auto">
          <HelpButton text={help} />
        </div>
      {/if}
    </header>
  {/if}
  <slot />
</section>
