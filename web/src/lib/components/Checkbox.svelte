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

<label class="label {classNames}" for={id}>
  <input {id} type="checkbox" bind:checked={value} />
  {label}
</label>
