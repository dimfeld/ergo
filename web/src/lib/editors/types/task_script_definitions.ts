import camelCase from 'just-camel-case';
import { Action, Input, TaskAction, TaskTrigger } from '../../api_types';

export interface ScriptDefinitionOptions {
  taskTriggers: Record<string, TaskTrigger>;
  taskActions: Record<string, TaskAction>;

  actions: Map<string, Action>;
  inputs: Map<string, Input>;
}

export function scriptTypeDefinitions({
  taskTriggers,
  taskActions,
  actions,
  inputs,
}: ScriptDefinitionOptions) {
  let actionFunctions = Object.entries(taskActions).map(([localId, action]) => {
    let payloadTypeName = camelCase(localId) + 'ActionPayload';

    let actionPayloadDef: string;
    let actionTemplate = actions.get(action.action_id)?.executor_template;
    if (actionTemplate?.t === 'Template') {
      actionPayloadDef = actionTemplate.c
        .map(([fieldName, fieldTemplate]) => {
          return `
      /** ${JSON.stringify(fieldTemplate)} */
      ${fieldName}: string;
      `;
        })
        .join('\n');
    } else {
      // Fallback for now when the template is done via script.
      // TODO Change the script type to include some type information.
      actionPayloadDef = `[key:string]: any;`;
    }

    return `
    interface ${payloadTypeName} {
      ${actionPayloadDef}
    }

    function runAction(name: '${localId}', payload: ${payloadTypeName}) : void;
    `;
  });

  let inputPayloadTypes =
    Object.entries(taskTriggers)
      .map(([localId, trigger]) => {
        let inputFields =
          Object.entries(inputs.get(trigger.input_id)?.payload_schema.properties ?? {})
            .map(([fieldName, fieldInfo]) => {
              // TODO Handle differences between JSON schema and Typescript types.
              // Also handle nested objects.
              return `${fieldName}: ${(fieldInfo as any).type};`;
            })
            .join('\n') || '[key: string]: any;';

        return `{
        trigger: '${camelCase(localId)}',
        data: {
          ${inputFields}
        }
      }`;
      })
      .join('\n| ') || 'never';

  return `
type InputPayload = ${inputPayloadTypes};

declare namespace Ergo {
  function getPayload(): InputPayload;

  ${actionFunctions.join('\n')}

  function getContext<CONTEXT>(): CONTEXT | undefined;
  function setContext<CONTEXT>(context: CONTEXT): void;
}

`;
}
