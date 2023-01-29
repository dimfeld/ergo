import Worker from './worker?worker';
import {
  workerShell,
  type SandboxHandlers,
  type SandboxWorker,
  type WorkerMessage,
} from '$lib/sandbox/messages_common';
import type { DataFlowManagerData } from '../dataflow_manager';
import type { DataFlowEdge } from '$lib/api_types';

export interface SandboxMessage {
  set_config: (config: WorkerMessage<SandboxWorkerData>) => Errors;
  run_all: (msg: WorkerMessage<null>) => Promise<RunResponse>;
  run_from: (msg: WorkerMessage<number>) => Promise<RunResponse>;
  update_node: (msg: WorkerMessage<UpdateNodeArgs>) => Errors;
  update_edges: (msg: WorkerMessage<DataFlowEdge[]>) => void;
}

export type SandboxWorkerData = Pick<
  DataFlowManagerData,
  'nodes' | 'edges' | 'toposorted' | 'nodeState'
>;

export interface NodeError {
  type: 'compile' | 'run';

  error: Error;
}

export interface Errors {
  /** From node ID to the error */
  nodes: Map<number, NodeError>;
}

export type NodeState = Map<number, unknown>;

export interface RunResponse {
  errors: Errors;
  state: NodeState;
  ran: number[];
}

export interface UpdateNodeArgs {
  id: number;
  name?: string;
  code?: string;
}

export interface DataflowSandboxWorker extends SandboxWorker<SandboxMessage> {
  setConfig(data: DataFlowManagerData): Promise<Errors>;
  initIfNeeded(data: DataFlowManagerData): Promise<Errors>;
  updateNode(args: UpdateNodeArgs): Promise<Errors>;
  updateEdges(edges: DataFlowEdge[]): Promise<void>;
  runAll(): Promise<RunResponse>;
  runFrom(id: number): Promise<RunResponse>;
}

export function sandboxWorker(handlers?: SandboxHandlers): DataflowSandboxWorker {
  let intf = workerShell<SandboxMessage>({ Worker, handlers });

  let needsInit = true;

  async function setConfig(data: DataFlowManagerData) {
    let workerData: SandboxWorkerData = {
      nodes: data.nodes,
      edges: data.edges,
      nodeState: data.nodeState,
      toposorted: data.toposorted,
    };

    return intf.sendMessage('set_config', workerData);
  }

  return {
    ...intf,
    setConfig,
    runAll: () => intf.sendMessage('run_all', null),
    runFrom: (id: number) => intf.sendMessage('run_from', id),
    updateNode: (args: UpdateNodeArgs) => intf.sendMessage('update_node', args),
    updateEdges: (edges: DataFlowEdge[]) => intf.sendMessage('update_edges', edges),
    initIfNeeded: async (data: DataFlowManagerData) => {
      if (!needsInit) {
        return;
      }

      try {
        return await setConfig(data);
      } catch (e) {
        needsInit = true;
        throw e;
      }
    },
  };
}
