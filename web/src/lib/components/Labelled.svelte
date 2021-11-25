<script context="module">
  let counter = 0;
</script>

<script lang="ts">
  export let label: string;

  let classNames: string | undefined = undefined;
  export { classNames as class };

  const labelFor = `labelled-${counter++}`;
</script>

<div class={classNames}>
  <label class="block label" for={labelFor}><slot name="label">{label}</slot></label>
  <div id={labelFor} class="mt-1">
    <slot />
  </div>
</div>
