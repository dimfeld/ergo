import type { DataFlowEdge } from '$lib/api_types';
import {
  initConsoleHandlers,
  initErrorHandlers,
  initMessageHandler,
  type WorkerMessage,
} from '$lib/sandbox/worker_common';
import groupBy from 'just-group-by';
import type { DataFlowManagerNode } from '../dataflow_manager';
import type { Errors, NodeError, RunResponse, SandboxMessage, SandboxWorkerData } from './messages';

initErrorHandlers();
initConsoleHandlers();

let config: SandboxWorkerData | null = null;

type Msg<T> = WorkerMessage<T>;

interface NodeFunction {
  func: (...args: unknown[]) => Promise<unknown>;
  inputIds: number[];
  inputNames: string[];
}

// This looks weird but is the MDN-approved way to get a reference to AsyncFunction.
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

interface WorkerState {
  nodeIdToIndex: Map<number, number>;
  nodeFunctions: Map<number, NodeFunction>;
  errors: Errors;
}

let workerState: WorkerState = {
  nodeIdToIndex: new Map(),
  nodeFunctions: new Map(),
  errors: {
    nodes: new Map(),
  },
};

function errorType(node: number): NodeError['type'] | null {
  let error = workerState.errors.nodes.get(node);
  return error?.type ?? null;
}

function createWorkerState() {
  // Gather node inputs
  let inputEdges = groupBy(config.edges, (e) => e.to);
  let nodeIdToIndex = new Map(config.nodes.map((n, i) => [n.meta.id, i]));

  let nodeFunctions = new Map<number, NodeFunction>();

  let errors: Errors = {
    nodes: new Map(),
  };

  for (let node of config.nodes) {
    let edges = inputEdges[node.meta.id];
    try {
      let f = createNodeFunction(node, edges);
      nodeFunctions.set(node.meta.id, f);
      if (errorType(node.meta.id) === 'compile') {
        errors.nodes.delete(node.meta.id);
      }
    } catch (e) {
      errors.nodes.set(node.meta.id, { type: 'compile', error: e });
    }
  }

  workerState = {
    nodeIdToIndex,
    nodeFunctions,
    errors,
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

function handleSetConfig(msg: Msg<SandboxWorkerData>) {
  config = msg.data;
  createWorkerState();
  return workerState.errors;
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

  return workerState.errors;
}

function handleUpdateEdges(msg: Msg<DataFlowEdge[]>) {
  config.edges = msg.data;
  createWorkerState();
  return workerState.errors;
}

/** Run all the nodes in topological order, accounting for the autorun setting. This is most useful when doing the initial load */
async function runAll(): Promise<RunResponse> {
  let nodesRan: number[] = [];
  for (let nodeIndex of config.toposorted) {
    let node = config.nodes[nodeIndex];
    if (node.meta.autorun) {
      let ran = await runOne(node);
      if (ran) {
        nodesRan.push(node.meta.id);
      }
    }
  }

  return {
    errors: workerState.errors,
    state: config.nodeState,
    ran: nodesRan,
  };
}

/** Run a node and its downstream nodes. */
async function runFrom(msg: Msg<number>) {
  const rootId = msg.data;
  let toRun = new Set([rootId]);
  let rootIndex = workerState.nodeIdToIndex.get(rootId);
  let nodesRan: number[] = [];

  let toposortedStart = config.toposorted.findIndex((n) => n === rootIndex);
  let toposorted = config.toposorted.slice(toposortedStart);

  for (let nodeIndex of toposorted) {
    let node = config.nodes[nodeIndex];
    if (!toRun.has(node.meta.id)) {
      continue;
    }

    let ran = await runOne(node);

    if (!ran) {
      continue;
    }

    nodesRan.push(node.meta.id);

    // Add the directly connected downstream nodes to the list of nodes to run.
    for (let edge of config.edges) {
      if (edge.from === node.meta.id) {
        let toNodeIndex = workerState.nodeIdToIndex.get(edge.to);
        if (config.nodes[toNodeIndex]?.meta.autorun) {
          toRun.add(edge.to);
        }
      }
    }
  }

  return {
    errors: workerState.errors,
    state: config.nodeState,
    ran: nodesRan,
  };
}

/** Run just a single node. Returns true if it ran successfully, false if it didn't. */
async function runOne(node: DataFlowManagerNode): Promise<boolean> {
  const nodeId = node.meta.id;
  if (errorType(nodeId) === 'compile') {
    return false;
  }

  const func = workerState.nodeFunctions.get(nodeId);

  // Gather the state
  let anyNull = false;
  let inputValues = func.inputIds.map((id) => {
    let state = config.nodeState.get(id) ?? null;
    if (state == null) {
      anyNull = true;
    }
    return state;
  });

  if (anyNull && !node.config.allow_null_inputs) {
    return false;
  }

  try {
    let result = await func.func(...inputValues);
    config.nodeState.set(node.meta.id, result);
    workerState.errors.nodes.delete(nodeId);
  } catch (e) {
    workerState.errors.nodes.set(nodeId, { type: 'run', error: e });
    return false;
  }

  return true;
}

initMessageHandler<SandboxMessage>({
  set_config: handleSetConfig,
  update_node: handleUpdateNode,
  update_edges: handleUpdateEdges,
  run_all: runAll,
  run_from: runFrom,
});
