<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import Loading from './Loading.svelte';
</script>

<Meta title="Primitives/Loading" component={Loading} />

<Template let:args>
  <div class="w-3/4 border border-gray-200 h-48">
    <Loading {...args} />
  </div>
</Template>

<Story name="Default" />
<Story name="Extra Delay" args={{ delay: 3000 }} />
<Story name="No Delay" args={{ delay: 0 }} />
<Story name="Large" args={{ heightClass: 'h-24' }} />
<Story name="Small" args={{ heightClass: 'h-6' }} />
