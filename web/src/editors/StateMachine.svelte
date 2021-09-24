<script lang="ts">
  import { StateMachine } from '^/api_types';
  import Editor from './Editor.svelte';
  import zip from 'just-zip-it';
  import { objectLinter, ObjectLintResult } from './lint';
  import prettier from 'prettier/standalone';
  import prettierBabel from 'prettier/parser-babel';
  import { TaskConfigValidator } from 'ergo-wasm';

  import stateMachineSchema from '^/../../schemas/state_machine.json';

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

  function prettierFormat(s: object | string) {
    return prettier.format(typeof s === 'string' ? s : JSON.stringify(s), {
      parser: 'json5',
      plugins: [prettierBabel],
    });
  }
</script>

<div class="flex flex-col space-y-4 h-full">
  {#each data as [compiled, source]}
    <div class="flex-1">
      <Editor
        format="json5"
        contents={source || prettierFormat(compiled)}
        linter={objectLinter(lint)}
        jsonSchema={stateMachineSchema}
      />
    </div>
  {/each}
</div>
