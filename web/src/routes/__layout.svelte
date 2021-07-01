<script lang="typescript">
  import '../app.postcss';
  import { createDarkStore } from '../styles';

  let darkModeStore = createDarkStore();
  $: darkMode = $darkModeStore ?? window.matchMedia('(prefers-color-scheme: dark)').matches;
</script>

<div class:dark={darkMode}>
  <slot />
</div>
