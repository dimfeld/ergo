<script lang="ts">
  import { StateMachine } from '$lib/api_types';
  import Editor from './Editor.svelte';
  import zip from 'just-zip-it';
  import { objectLinter, ObjectLintResult } from './lint';
  import prettier from 'prettier/standalone';
  import prettierBabel from 'prettier/parser-babel';
  import { TaskConfigValidator } from 'ergo-wasm';

  import stateMachineSchema from '$lib/../../../schemas/state_machine.json';
  import { EditorView } from '@codemirror/view';
  import { json5ParseCache } from './codemirror-json5';

  interface Source {
    config: string;
  }

  export let compiled: StateMachine[];
  export let source: Source[];
  export let validator: TaskConfigValidator;

  let editors: EditorView[] = [];
  export function getState() {
    let sources = editors.map((view, i) => {
      // TODO A way to return a message if the source is not compilable.
      // TODO Run the validator and/or verify there are no diagnostics.
      let parsed = view.state.field(json5ParseCache);

      return {
        source: {
          type: 'StateMachine',
          data: {
            config: view.state.doc.toString(),
          },
        },
        compiled: {
          type: 'StateMachine',
          data: parsed?.obj ?? compiled[i],
        },
      };
    });

    return {
      compiled: sources.map((s) => s.compiled),
      source: sources.map((s) => s.source),
    };
  }

  $: data = zip(compiled || [], source || []) as [StateMachine, Source][];

  $: lint = (obj: StateMachine): ObjectLintResult[] => {
    let vals = validator?.validate_config({ type: 'StateMachine', data: [obj] }) ?? [];
    console.log('linting', obj, vals);

    // Remove the leading 'data[0]' from each path since we inserted it above.
    for (let v of vals) {
      if (v.path?.[0] === 'data') {
        v.path.shift();
        v.path.shift();
      }
    }

    return vals;
  };

  function prettierFormat(s: object | string) {
    return prettier.format(typeof s === 'string' ? s : JSON.stringify(s), {
      parser: 'json5',
      plugins: [prettierBabel],
    });
  }
</script>

<div class="flex flex-col space-y-4 h-full">
  {#each data as [compiled, source], i}
    <div class="flex-1">
      <Editor
        format="json5"
        contents={source?.config || prettierFormat(compiled)}
        linter={objectLinter(lint)}
        jsonSchema={stateMachineSchema}
        bind:view={editors[i]}
      />
    </div>
  {/each}
</div>