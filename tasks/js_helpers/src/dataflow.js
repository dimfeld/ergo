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
  let s = stringify(state);
  nodeState[nodeName] = s;
  return s;
}

export async function runNode(nodeName, nodeNamespace, nodeFunc) {
  let nodeFn = globalThis[nodeNamespace][nodeFunc];

  let state_result = await nodeFn(nodeState);
  nodeState[nodeName] = state_result;
  return stringify(state_result);
}
