<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import Loading from './Loading.svelte';
</script>

<Meta title="Primitives/Loading" component={Loading} />

<Template>
  <div class="w-3/4 border border-gray-200 h-48">
    <Loading />
  </div>
</Template>

<Story id="loader_story" name="Loading" />
