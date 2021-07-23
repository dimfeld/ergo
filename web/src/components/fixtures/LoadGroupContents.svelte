<script lang="ts">
  import { getContext } from 'svelte';
  import { autoregisterLoadGroup } from '../loadGroup';
  export let name: string;

  let groupStatus = getContext(name);
  autoregisterLoadGroup(groupStatus, name);
</script>

<div>
  {name} loaded
</div>
