<script lang="ts">
  import { setContext } from 'svelte';
  import LoadGroup from '../LoadGroup.svelte';
  import LoadGroupContents from './LoadGroupContents.svelte';

  export let parentGroupProps = {};
  export let childGroupProps = {};

  export let a;
  export let b;
  export let c;

  setContext('a', a);
  setContext('b', b);
  setContext('c', c);
</script>

<LoadGroup name="parent" {...parentGroupProps}>
  <div slot="loading">Parent group loading</div>
  <div slot="error" let:error>Parent group error: {error.message}</div>

  <LoadGroupContents name="a" />
  <LoadGroupContents name="b" />
  <LoadGroup name="child" {...childGroupProps}>
    <div slot="loading">Child group loading</div>
    <div slot="error" let:error>Child group error: {error.message}</div>

    <LoadGroupContents name="c" />
  </LoadGroup>
</LoadGroup>
