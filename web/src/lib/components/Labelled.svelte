<script context="module">
  let counter = 0;
</script>

<script lang="ts">
  export let label: string;
  export let help: string | null | undefined = undefined;
  export let big = false;

  let classNames: string | undefined = undefined;
  export { classNames as class };

  const labelFor = `labelled-${counter++}`;
</script>

<div class={classNames}>
  <div class="label flex items-end justify-between">
    <label for={labelFor} class:big-label={big}><slot name="label">{label}</slot></label>
    {#if help || $$slots.help}
      <span class="ml-8"><slot name="help">{help}</slot></span>
    {/if}
  </div>
  <div id={labelFor} class="mt-1">
    <slot />
  </div>
</div>
