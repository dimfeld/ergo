import type { DataFlowEdge } from '$lib/api_types';
import {
  initConsoleHandlers,
  initErrorHandlers,
  initMessageHandler,
  type WorkerMessage,
} from '$lib/sandbox/worker_common';
import groupBy from 'just-group-by';
import type { DataFlowManagerData, DataFlowManagerNode } from '../dataflow_manager';
import type { SandboxMessageName } from './messages';

initErrorHandlers();
initConsoleHandlers();

let config: DataFlowManagerData | null = null;
let nodeState: Map<number, any> = new Map();

type Msg<T> = WorkerMessage<SandboxMessageName, T>;

interface NodeFunction {
  func: () => Promise<any>;
  inputIds: number[];
  inputNames: string[];
}

// This looks weird but is the MDN-approved way to get a reference to AsyncFunction.
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

let workerState = {
  nodeIdToIndex: new Map<number, number>(),
  nodeFunctions: new Map<number, NodeFunction>(),
};

function createWorkerState() {
  // Gather node inputs
  let inputEdges = groupBy(config.edges, (e) => e.to);
  let nodeIdToIndex = new Map(config.nodes.map((n, i) => [n.meta.id, i]));

  let nodeFunctions = new Map<number, NodeFunction>();

  for (let node of config.nodes) {
    let edges = inputEdges[node.meta.id];
    let f = createNodeFunction(node, edges);
    nodeFunctions.set(node.meta.id, f);
  }

  workerState = {
    nodeIdToIndex,
    nodeFunctions,
  };
}

function createNodeFunction(node: DataFlowManagerNode, edges: DataFlowEdge[]) {
  let inputIds = edges.map((i) => i.from);
  let inputNames = inputIds.map((id) => {
    let node = config.nodes.find((n) => n.meta.id === id);
    return node.config.name;
  });

  let func = compile(node, inputNames);

  return {
    func,
    inputIds,
    inputNames,
  };
}

function compile(node: DataFlowManagerNode, inputs: string[]) {
  let code = node.meta.contents;
  if (node.meta.format === 'expression') {
    code = 'return ' + code;
  }

  return new AsyncFunction(...inputs, code);
}

function handleSetConfig(msg: Msg<DataFlowManagerData>) {
  config = msg.data;
  createWorkerState();
}

function handleInitState(msg: Msg<Map<number, any>>) {
  nodeState = msg.data;
}

function handleUpdateNode(msg: Msg<{ id: number; name?: string; code?: string }>) {
  const { id, name, code } = msg.data;
  let node = config.nodes.find((n) => n.meta.id === id);
  if (!node) {
    throw new Error(`could not find node ${id}`);
  }

  let recompileOne = false;
  let updateAll = false;

  if (name && node.config.name !== name) {
    node.config.name = name;
    // Since a node name changed, we need to change the name of the argument in every
    // function that uses it.
    updateAll = true;
  }

  if (code && node.meta.contents !== code) {
    recompileOne = true;
    node.meta.contents = code;
  }

  if (updateAll) {
    createWorkerState();
  } else if (recompileOne) {
    let existingState = workerState.nodeFunctions.get(id);
    let newFunc = compile(node, existingState.inputNames);
    existingState.func = newFunc;
  }
}

function handleUpdateEdges(msg: Msg<DataFlowEdge[]>) {
  config.edges = msg.data;
  createWorkerState();
}

initMessageHandler<SandboxMessageName>({
  set_config: handleSetConfig,
  init_state: handleInitState,
  update_node: handleUpdateNode,
  update_edges: handleUpdateEdges,
});
