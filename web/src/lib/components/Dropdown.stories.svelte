<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';

  import DropdownWithMenu from './fixtures/DropdownWithMenu.svelte';
  import Dropdown from './Dropdown.svelte';
  import ChevronRight from './icons/ChevronRight.svelte';
</script>

<Meta title="Primitives/Dropdown" component={Dropdown} />

<Template let:args>
  <DropdownWithMenu {...args} />
</Template>

<Story name="Default" args={{ label: 'Menu' }} />

<Story name="Right Arrow" args={{ label: 'Menu', position: 'right-start', arrow: ChevronRight }} />

<Story name="No arrow" args={{ label: 'Menu', arrow: false }} />
