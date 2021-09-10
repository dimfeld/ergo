<script lang="ts">
  import { getContext } from 'svelte';
  import { Readable } from 'svelte/store';
  import { autoregisterLoadGroup, LoadGroupStoreData } from '../loadGroup';
  export let name: string;

  let groupStatus = getContext<Readable<LoadGroupStoreData>>(name);
  autoregisterLoadGroup(groupStatus, name);
</script>

<div>
  {name} loaded
</div>
