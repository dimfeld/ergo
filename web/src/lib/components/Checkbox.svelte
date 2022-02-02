<script context="module">
  let checkboxId = 0;
</script>

<script lang="ts">
  export let value: boolean | undefined;
  export let label: string;
  let classNames = '';
  export { classNames as class };

  let id = `checkbox-${checkboxId++}`;
</script>

<label class="label flex items-center {classNames}" for={id}>
  <input {id} class="mr-2" type="checkbox" bind:checked={value} />
  <slot>{label}</slot>
</label>
