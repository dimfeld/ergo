<script lang="ts">
  import { baseData } from '$lib/data';
  import { getHeaderTextStore } from '$lib/header';
  const { inputs } = baseData();
  getHeaderTextStore().set(['Inputs']);
</script>

<ul class="space-y-4">
  {#each Array.from($inputs.values()) as input (input.input_id)}
    <li>
      <p>
        {input.name}{#if input.description} &mdash; {input.description}{/if}
      </p>
      <ul class="ml-4">
        {#each Object.entries(input.payload_schema.properties) as [field, fieldType] (field)}
          <li>
            <span class="text-gray-800 dark:text-gray-200 font-medium">{field}</span>: {fieldType.type}
            {#if fieldType.format}
              in {fieldType.format} format{/if}
          </li>
        {/each}
      </ul>
    </li>
  {/each}
</ul>
