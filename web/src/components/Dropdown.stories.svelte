<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';

  import Dropdown from './Dropdown.svelte';
  import MenuItem from './MenuItem.svelte';
</script>

<Meta title="Dropdown" component={Dropdown} />

<Template let:args>
  <Dropdown {...args}>
    <div class="flex flex-col">
      <MenuItem>Dropdown contents</MenuItem>
      <MenuItem>More dropdown contents</MenuItem>
    </div>
  </Dropdown>
</Template>

<Story name="Default" args={{ label: 'Menu' }} />

<Story name="No arrow" args={{ label: 'Menu', arrow: false }} />
