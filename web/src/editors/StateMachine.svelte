<script lang="ts">
  import { StateMachine } from '^/api_types';
  import Editor from './Editor.svelte';
  import EventHandler from './state_machine/EventHandler.svelte';
  import zip from 'just-zip-it';

  export let compiled: StateMachine[];
  export let source: string[];
  // This is totally unfinished but shows a very basic outline of the state machine.

  $: data = zip(compiled || [], source || []);
</script>

<div class="flex flex-col space-y-4">
  {#each data as [compiled, source]}
    <div class="flex-1 grid grid-rows-1 grid-cols-1 place-items-stretch">
      <!-- <p>State Machine <strong>{machine.name}</strong></p> -->
      <!-- {#if machine.description} -->
      <!--   <p>{machine.description}</p> -->
      <!-- {/if} -->
      <!-- <p>Initial State: {machine.initial}</p> -->

      <!-- <p>Global Handlers</p> -->
      <!-- {#each machine.on as on} -->
      <!--   <div class="ml-4"> -->
      <!--     <EventHandler handler={on} /> -->
      <!--   </div> -->
      <!-- {/each} -->

      <!-- <p>States</p> -->
      <!-- {#each Object.entries(machine.states) as [name, state] (name)} -->
      <!--   <div class="ml-4"> -->
      <!--     <p> -->
      <!--       <span class="font-bold text-accent-700 dark:text-accent-200">{name}</span -->
      <!--       >{#if state.description} - {state.description}{/if} -->
      <!--     </p> -->
      <!--     {#each state.on as on} -->
      <!--       <div class="ml-4"> -->
      <!--         <EventHandler handler={on} /> -->
      <!--       </div> -->
      <!--     {/each} -->
      <!--   </div> -->
      <!-- {/each} -->
      <Editor format="json" contents={source || JSON.stringify(compiled)} />
    </div>
  {/each}
</div>
