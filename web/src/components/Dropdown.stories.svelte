<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';

  import Dropdown from './Dropdown.svelte';
</script>

<Meta title="Dropdown" component={Dropdown} />

<Template let:args>
  <Dropdown {...args}>
    <p>Dropdown contents</p>
    <p>More dropdown contents</p>
  </Dropdown>
</Template>

<Story name="Default" args={{ label: 'Menu' }} />

<Story name="No arrow" args={{ label: 'Menu', arrow: false }} />
