<script context="module">
  let checkboxId = 0;
</script>

<script lang="ts">
  export let value: boolean;
  export let label: string;

  let id = `checkbox-${checkboxId++}`;
</script>

<label class="label" for={id}>
  <input {id} type="checkbox" bind:checked={value} />
  {label}
</label>
