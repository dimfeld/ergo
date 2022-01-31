<script lang="ts">
  import HelpButton from './HelpButton.svelte';

  let classNames: string | undefined = undefined;
  export { classNames as class };

  export let label = '';
  export let help = '';
</script>

<section
  class="w-full rounded border border-gray-200 p-4 shadow-md dark:border-gray-400 {classNames ||
    ''}"
>
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
