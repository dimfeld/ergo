<script lang="ts">
  import { getContext } from 'svelte';
  import type { Readable } from 'svelte/store';
  import { autoregisterLoadGroup, type LoadGroupStoreData } from '../loadGroup';
  export let name: string;

  let groupStatus = getContext<Readable<LoadGroupStoreData>>(name);
  autoregisterLoadGroup(groupStatus, name);
</script>

<div>
  {name} loaded
</div>
