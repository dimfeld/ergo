import Worker from './worker?worker';
import {
  workerShell,
  type SandboxHandlers,
  type SandboxWorker,
} from '$lib/sandbox/messages_common';
import type { DataFlowManagerData } from '../dataflow_manager';
import type { DataFlowEdge } from '$lib/api_types';

export type SandboxMessageName =
  | 'set_config'
  | 'update_node'
  | 'update_edges'
  | 'run_all'
  | 'run_one';

export type SandboxWorkerData = Pick<
  DataFlowManagerData,
  'nodes' | 'edges' | 'toposorted' | 'nodeState'
>;

export interface UpdateNodeArgs {
  id: number;
  name?: string;
  code?: string;
}

export interface DataflowSandboxWorker extends SandboxWorker {
  setConfig(data: DataFlowManagerData): Promise<void>;
  initIfNeeded(data: DataFlowManagerData): Promise<void>;
  updateNode(args: UpdateNodeArgs): Promise<void>;
  updateEdges(edges: DataFlowEdge[]): Promise<void>;
  runAll(): Promise<Map<number, unknown>>;
  runOne(id: number): Promise<Map<number, unknown>>;
}

export function sandboxWorker(handlers: SandboxHandlers): DataflowSandboxWorker {
  let intf = workerShell<SandboxMessageName>({ Worker, handlers });

  let needsInit = true;

  async function setConfig(data: DataFlowManagerData) {
    let workerData: SandboxWorkerData = {
      nodes: data.nodes,
      edges: data.edges,
      nodeState: data.nodeState,
      toposorted: data.toposorted,
    };

    return intf.sendMessage<void>('set_config', workerData);
  }

  return {
    ...intf,
    setConfig,
    runAll: () => intf.sendMessage('run_all', null),
    runOne: (id: number) => intf.sendMessage('run_one', id),
    updateNode: (args: UpdateNodeArgs) => intf.sendMessage('update_node', args),
    updateEdges: (edges: DataFlowEdge[]) => intf.sendMessage('update_edges', edges),
    initIfNeeded: async (data: DataFlowManagerData) => {
      if (!needsInit) {
        return;
      }

      try {
        await setConfig(data);
      } catch (e) {
        needsInit = true;
        throw e;
      }
    },
  };
}
