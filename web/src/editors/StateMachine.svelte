<script lang="ts">
  import { StateMachine } from '^/api_types';
  import Editor from './Editor.svelte';
  import zip from 'just-zip-it';
  import { objectLinter, ObjectLintResult } from './lint';
  import prettier from 'prettier/standalone';
  import prettierBabel from 'prettier/parser-babel';
  import { TaskConfigValidator } from 'ergo-wasm';

  export let compiled: StateMachine[];
  export let source: string[];
  export let validator: TaskConfigValidator;
  // This is totally unfinished but shows a very basic outline of the state machine.

  $: data = zip(compiled || [], source || []) as [StateMachine, string][];

  function lint(obj: StateMachine): ObjectLintResult[] {
    let vals = validator?.validate_config({ type: 'StateMachine', data: [obj] }) ?? [];

    // Remove the leading 'data[0]' from each path since we inserted it above.
    for (let v of vals) {
      if (v.path?.[0] === 'data') {
        v.path.shift();
        v.path.shift();
      }
    }

    return vals;
  }
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
      <Editor
        format="json5"
        contents={source ||
          prettier.format(JSON.stringify(compiled), { parser: 'json5', plugins: [prettierBabel] })}
        linter={objectLinter(lint)}
      />
    </div>
  {/each}
</div>
