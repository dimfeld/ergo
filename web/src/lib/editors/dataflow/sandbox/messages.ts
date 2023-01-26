import Worker from './worker?worker';
import {
  workerShell,
  type SandboxHandlers,
  type SandboxWorker,
} from '$lib/sandbox/messages_common';
import type { DataFlowManagerData } from '../dataflow_manager';
import type { DataFlowEdge } from '$lib/api_types';

export type SandboxMessageName = 'init_state' | 'set_config' | 'update_node' | 'update_edges';

export type SandboxWorkerData = Pick<DataFlowManagerData, 'nodes' | 'edges' | 'toposorted'>;

export interface UpdateNodeArgs {
  id: number;
  name?: string;
  code?: string;
}

export interface DataflowSandboxWorker extends SandboxWorker {
  setConfig(data: DataFlowManagerData): Promise<void>;
  initIfNeeded(data: DataFlowManagerData, state: Map<number, any>): Promise<void>;
  updateNode(args: UpdateNodeArgs): Promise<void>;
  updateEdges(edges: DataFlowEdge[]): Promise<void>;
}

export function sandboxWorker(handlers: SandboxHandlers): DataflowSandboxWorker {
  let intf = workerShell<SandboxMessageName>({ Worker, handlers });

  let needsInit = true;

  async function setConfig(data: DataFlowManagerData) {
    let workerData = {
      nodes: data.nodes,
      edges: data.edges,
      toposorted: data.toposorted,
    };

    return intf.sendMessage<void>('set_config', workerData);
  }

  function initState(state: Map<number, object>) {
    return intf.sendMessage<void>('init_state', state);
  }

  return {
    ...intf,
    setConfig,
    updateNode: (args: UpdateNodeArgs) => intf.sendMessage('update_node', args),
    updateEdges: (edges: DataFlowEdge[]) => intf.sendMessage('update_edges', edges),
    initIfNeeded: async (data: DataFlowManagerData, state: Map<number, any>) => {
      if (!needsInit) {
        return;
      }

      try {
        await Promise.all([setConfig(data), initState(state)]);
      } catch (e) {
        needsInit = true;
        throw e;
      }
    },
  };
}
