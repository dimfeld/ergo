import type { DataFlowConfig, DataFlowEdge, DataFlowNode } from '$lib/api_types';
import zip from 'just-zip-it';
import { get as getStore, writable } from 'svelte/store';
import type { Box } from '../canvas/drag';
import { toposort_nodes } from 'ergo-wasm';
import groupBy from 'just-group-by';
import { schemeOranges } from 'd3';

export type JsFunctionType = 'expression' | 'function';

/** Metadata used only for the front-end. */
export interface DataFlowNodeMeta {
  id: number;
  position: Box;
  splitPos: number;
  autorun: boolean;
  lastOutput: string;
  edgeColor: string;
  format: JsFunctionType;
  contents: string;
}

export const DEFAULT_FUNC_TYPE: JsFunctionType = 'expression';

export interface DataFlowSource {
  nodes: DataFlowNodeMeta[];
}

export interface DataFlowManagerNode {
  config: DataFlowNode;
  meta: DataFlowNodeMeta;
}

export interface DataFlowManagerData {
  nodes: DataFlowManagerNode[];
  edges: DataFlowEdge[];
  toposorted: number[];
  edgesByDestination: Record<number, DataFlowEdge[]>;
  nodeById: (id: number) => DataFlowManagerNode;
  nodeIdToIndex: Map<number, number>;
  checkAddEdge: (from: number, to: number) => string | null;
}

export function wrapFunction(func: string, funcType: JsFunctionType) {
  if (funcType === 'expression') {
    return `return ${func}`;
  }

  return func;
}

function findLeastUsedColor(colors: readonly string[], nodes: DataFlowManagerNode[]) {
  let colorCounts = new Map<string, number>(colors.map((c) => [c, 0]));
  for (let node of nodes) {
    if (node.meta.edgeColor) {
      colorCounts.set(node.meta.edgeColor, colorCounts.get(node.meta.edgeColor) + 1);
    }
  }
  let min = Number.MAX_SAFE_INTEGER;
  let minColor = colors[0];
  for (let [color, count] of colorCounts) {
    if (count < min) {
      min = count;
      minColor = color;
    }
  }
  return minColor;
}

export function dataflowManager(config: DataFlowConfig, source: DataFlowSource) {
  let colors = schemeOranges[9];

  const nodes = zip(config?.nodes || [], source?.nodes || []).map(([config, meta], i) => {
    if (!meta.edgeColor) {
      meta.edgeColor = colors[i % colors.length];
    }

    return {
      config,
      meta,
    };
  });

  function generateLookups(nodes: DataFlowManagerNode[], edges: DataFlowEdge[]) {
    let nodeIdToIndex = new Map<number, number>(nodes.map((node, i) => [node.meta.id, i]));
    let edgesForSort = edges.map((edge) => ({
      ...edge,
      to: nodeIdToIndex.get(edge.to),
      from: nodeIdToIndex.get(edge.from),
    }));
    let toposorted = toposort_nodes(nodes.length, edgesForSort);

    const edgesByDestination = groupBy(edges, (edge) => edge.to);

    const checkAddEdge = (from: number, to: number) => {
      if (edges.find((e) => e.from === from && e.to === to)) {
        return 'Edge already exists';
      }

      let seen = new Set([from]);

      // Return true if there is a cycle, defined by arriving back at the original "from" node.
      const findCycle = (node: number) => {
        // This has bad O but the data size is small.
        for (let edge of edges) {
          if (edge.from !== node) {
            continue;
          }

          if (edge.to === from) {
            return true;
          }

          if (seen.has(edge.to)) {
            return false;
          }

          seen.add(edge.to);

          if (findCycle(edge.to) === true) {
            return true;
          }
        }

        return false;
      };

      // Starting from the "to" node, traverse the graph to see if we can reach the "from" node.
      const hasCycle = findCycle(to);

      return hasCycle ? 'Adding this edge causes a cycle' : null;
    };

    const nodeById = (id: number) => nodes[nodeIdToIndex.get(id)];

    return { nodeIdToIndex, nodeById, edgesByDestination, toposorted, checkAddEdge };
  }

  // We use the node IDs so it's easier to move things around, but the version on the backend uses indexes.
  let edges = (config?.edges || [])
    .map((edge) => ({
      ...edge,
      from: nodes[edge.from]?.meta.id ?? -1,
      to: nodes[edge.to]?.meta.id ?? -1,
    }))
    .filter((e) => e.from !== -1 && e.to !== -1);

  let lookups = generateLookups(nodes, edges);

  let store = writable({
    nodes,
    edges,
    ...lookups,
  });

  function update(updateFn: (data: DataFlowManagerData) => DataFlowManagerData) {
    store.update((data) => {
      let result = updateFn(data);
      let lookups = generateLookups(result.nodes, result.edges);
      return {
        ...result,
        ...lookups,
      };
    });
  }

  return {
    subscribe: store.subscribe,
    set: store.set,
    update,
    compile(): { compiled: DataFlowConfig; source: DataFlowSource } {
      let data = getStore(store);

      let nodeConfig = data.nodes.map((n) => n.config);
      let nodeSource = data.nodes.map((n) => n.meta);

      // Convert edges back from node IDs to indexes.
      let edges = data.edges
        .map((e) => ({
          ...e,
          from: data.nodeIdToIndex.get(e.from) ?? -1,
          to: data.nodeIdToIndex.get(e.to) ?? -1,
        }))
        .filter((e) => e.from !== -1 && e.to !== -1);

      return {
        compiled: {
          nodes: nodeConfig,
          edges,
          compiled: '', // TODO compile it
          map: null,
          toposorted: data.toposorted,
        },
        source: {
          nodes: nodeSource,
        },
      };
    },
    addEdge(from: number, to: number) {
      update((data) => {
        let existingEdge = data.edges.find((e) => e.from === from && e.to === to);
        if (existingEdge) {
          return data;
        }

        if (!data.nodeIdToIndex.has(from)) {
          throw new Error('from node does not exist');
        }

        if (!data.nodeIdToIndex.has(to)) {
          throw new Error('to node does not exist');
        }

        let edges: DataFlowEdge[] = [
          ...data.edges,
          {
            from,
            to,
          },
        ];

        return {
          ...data,
          edges,
        };
      });
    },
    deleteEdge(from: number, to: number) {
      update((data) => {
        let edges = data.edges.filter((e) => e.from !== from || e.to !== to);
        return {
          ...data,
          edges,
        };
      });
    },
    addNode(box: Box) {
      update((data) => {
        let newNodeIndex = data.nodes.length;
        let newNodeName = `node${newNodeIndex}`;
        while (data.nodes.some((n) => n.config.name === newNodeName)) {
          newNodeIndex += 1;
          newNodeName = `node${newNodeIndex}`;
        }

        let maxId = Math.max(...data.nodes.map((n) => n.meta.id), 0);

        let nodes: DataFlowManagerNode[] = [
          ...data.nodes,
          {
            config: {
              allow_null_inputs: true,
              name: newNodeName,
              func: {
                type: 'js',
                func: `__${newNodeName}`,
              },
            },
            meta: {
              id: maxId + 1,
              position: box,
              splitPos: 75,
              autorun: true,
              lastOutput: '',
              edgeColor: findLeastUsedColor(colors, data.nodes),
              format: 'expression',
              contents: '',
            },
          },
        ];

        return {
          ...data,
          nodes,
        };
      });
    },
    deleteNode(id: number) {
      update((data) => {
        let index = data.nodeIdToIndex.get(id);
        if (typeof index !== 'number') {
          return data;
        }

        let nodeId = data.nodes[index].meta.id;
        let edges = data.edges.filter((e) => e.from !== nodeId && e.to !== nodeId);
        let nodes = data.nodes.filter((n) => n.meta.id !== id);

        return {
          ...data,
          edges,
          nodes,
        };
      });
    },
  };
}
