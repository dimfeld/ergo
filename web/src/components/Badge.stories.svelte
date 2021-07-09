<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import Badge from './Badge.svelte';
</script>

<Meta title="Primitives/Badge" component={Badge} />

<Template>
  {#each ['', 'dark bg-gray-900'] as className}
    <div class="p-4 flex flex-row space-x-4 {className}">
      <Badge style="plain">Plain</Badge>
      <Badge style="accent">Accent</Badge>
      <Badge style="success">Success</Badge>
      <Badge style="error">Error</Badge>
    </div>
  {/each}
</Template>

<Story name="Text" />
