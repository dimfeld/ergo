import { stringify, parse } from 'devalue' ;

let nodeState = {};

export function initState(stateObj) {
  nodeState = {}
  for(let [id, value] of Object.entries(stateObj)) {
    nodeState[id] = value ? parse(value) : null;
  }
}

export function serializeState() {
  let output = {};
  for(let [id, value] of Object.entries(nodeState)) {
    output[id] = stringify(value);
  }
  return output;
}

export function getState(nodeName) {
  return nodeState[nodeName];
}

export function setNodeState(nodeName, state) {
  nodeState[nodeName] = state;
  return stringify(state);
}

export async function runNode(nodeName, nodeNamespace, nodeFunc, nullCheckNodes) {
  let nodeFn = globalThis[nodeNamespace][nodeFunc];

  if(Array.isArray(nullCheckNodes)) {
    for(let node of nullCheckNodes) {
      // Skip on null or undefined. Empty string indicates that nothing happened.
      if(nodeState[node] == null) {
        return '';
      }
    }
  }

  let state_result = await nodeFn(nodeState);
  nodeState[nodeName] = state_result;
  return stringify(state_result);
}
