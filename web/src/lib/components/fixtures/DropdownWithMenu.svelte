<script lang="ts">
  import Dropdown from '../Dropdown.svelte';
  import MenuItem from '../MenuItem.svelte';
</script>

<div class="h-32 bg-transparent">
  <Dropdown {...$$props}>
    <div class="flex flex-col">
      <MenuItem>Dropdown contents</MenuItem>
      <MenuItem>More dropdown contents</MenuItem>
    </div>
  </Dropdown>
</div>
