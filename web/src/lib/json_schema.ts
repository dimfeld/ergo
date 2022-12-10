import type { JSONSchema4 } from 'json-schema';

export function defaultFromJsonSchema(schema: JSONSchema4) {
  let output: Record<string, any> = {};
  for (let [propertyName, desc] of Object.entries(schema.properties ?? {})) {
    if (desc.$ref) {
      // TODO ref support
      output[propertyName] = {};
    } else {
      let type = Array.isArray(desc.type) ? desc.type[0] : desc.type;
      switch (type) {
        case 'string':
          output[propertyName] = '';
          break;
        case 'number':
        case 'integer':
          output[propertyName] = 0;
          break;
        case 'object':
          output[propertyName] = {};
          break;
        case 'array':
          output[propertyName] = [];
          break;
        case 'boolean':
          output[propertyName] = false;
          break;
        default:
          output[propertyName] = null;
      }
    }
  }

  return output;
}

export const stringFormats = ['date-time', 'time', 'date', 'email', 'uuid', 'uri'];
