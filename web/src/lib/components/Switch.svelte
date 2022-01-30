<script lang="ts">
  export let value: boolean;
  export let name: string;
  export let disabled = false;
</script>

<label class="relative inline-flex items-center">
  <input type="checkbox" {name} {disabled} class="w-0 h-0 opacity-0" bind:checked={value} />
  <div
    class="slider-bg inset-0 inline-flex flex-shrink-0 bg-gray-200 dark:bg-gray-600 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 ring-offset-dwhite"
    class:mr-2={$$slots.default}
  >
    <span
      class="slider pointer-events-none inline-block cursor-pointer bg-white dark:bg-gray-900 h-5 w-5 rounded-full shadow ring-0 transition ease-in-out duration-200"
    />
  </div>
  <slot />
</label>

<style lang="postcss">
  input:checked ~ .slider-bg {
    @apply bg-daccent-500;
  }

  input:focus ~ .slider-bg {
    @apply outline-none ring-2 ring-offset-2 ring-daccent-500;
  }

  input:checked + .slider-bg > .slider {
    @apply translate-x-5;
  }
</style>
