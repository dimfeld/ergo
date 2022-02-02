import camelCase from 'just-camel-case';
import { Action, Input, TaskAction, TaskTrigger } from '../../api_types';

export interface ScriptDefinitionOptions {
  taskTriggers: Record<string, TaskTrigger>;
  taskActions: Record<string, TaskAction>;

  actions: Map<string, Action>;
  inputs: Map<string, Input>;
}

function pluralValues(v: number) {
  return v === 1 ? 'value' : 'values';
}

export function scriptTypeDefinitions({
  taskTriggers,
  taskActions,
  actions,
  inputs,
}: ScriptDefinitionOptions) {
  let actionFunctions = Object.entries(taskActions).map(([localId, action]) => {
    let payloadTypeName = camelCase(localId) + 'ActionPayload';

    let actionTemplate = actions.get(action.action_id)?.template_fields;
    let actionPayloadDef = (actionTemplate ?? [])
      .map((field) => {
        let tsType = field.format.type as string;
        let constraints = '';
        switch (field.format.type) {
          case 'choice':
            {
              let choiceString = field.format.choices.join(', ');
              let { min, max } = field.format;

              if (min && max) {
                if (min === max) {
                  constraints = `${min} ${pluralValues(min)} from ${choiceString}`;
                } else {
                  constraints = `${min} to ${max} values from ${choiceString}`;
                }
              } else if (min) {
                constraints = `At least ${min} ${pluralValues(min)} from ${choiceString}`;
              } else if (max) {
                constraints = `At most ${max} ${pluralValues(max)} from ${choiceString}`;
              } else {
                constraints = `Any number of values from ${choiceString}`;
              }

              tsType = 'string[]';
            }
            break;
          case 'string_array':
            tsType = 'string[]';
            break;
          case 'float':
          case 'integer':
            tsType = 'number';
            break;
        }

        let commentContents = [
          field.description,
          field.optional ? `@default ${field.format.default}` : null,
          constraints,
        ]
          .filter(Boolean)
          .join('\n');

        let comment = commentContents ? `/** ${commentContents} */` : '';
        let fieldDef = `${field.name}${field.optional ? '?' : ''}: ${tsType};`;

        return [comment, fieldDef].filter(Boolean).join('\n');
      })
      .join('\n\n');

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
