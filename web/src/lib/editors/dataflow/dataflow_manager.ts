import type {
  DataFlowConfig,
  DataFlowEdge,
  DataFlowNode,
  DataFlowNodeFunction,
  TaskTrigger,
} from '$lib/api_types';
import zip from 'just-zip-it';
import { get as getStore, writable } from 'svelte/store';
import type { Box } from '../canvas/drag';
import { toposort_nodes } from 'ergo-wasm';
import groupBy from 'just-group-by';
import { schemeOranges } from 'd3';
import type { Bundler } from '$lib/bundler';

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

async function compileCode(
  bundler: Bundler,
  data: DataFlowManagerData
): Promise<{
  compiled: DataFlowConfig;
  source: DataFlowSource;
}> {
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

  let functions = data.nodes.map((node, i) => wrapFunction(data, edges, i, node)).join('\n');

  let bundled = await bundler.bundle({
    files: {
      'index.js': functions,
    },
    name: '',
    format: 'iife',
    production: true,
  });

  if (bundled.type === 'error') {
    // TODO show the error somewhere
    throw bundled.error;
  }

  return {
    compiled: {
      nodes: nodeConfig,
      edges,
      compiled: bundled.code,
      map: bundled.map ? JSON.stringify(bundled.map) : undefined,
      toposorted: data.toposorted,
    },
    source: {
      nodes: nodeSource,
    },
  };
}

export function wrapFunction(
  data: DataFlowManagerData,
  indexEdges: DataFlowEdge[],
  nodeIndex: number,
  node: DataFlowManagerNode
) {
  const nodeType = node.config.func.type;
  if (nodeType === 'js' || nodeType === 'action') {
    let contents = node.meta.contents;
    if (node.meta.format === 'expression') {
      contents = `return ${contents}`;
    }

    let inputNodes = indexEdges
      .filter((edge) => edge.to === nodeIndex)
      .map(({ from }) => data.nodes[from].config.name);

    let funcName = nodeType === 'js' ? node.config.func.func : node.config.func.payload_code.func;

    let args = inputNodes.length ? `{ ${inputNodes.join(',')} }` : '';

    return `export async function ${funcName}(${args}) {
      ${contents}
    };`;
  }
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

export function dataflowManager(bundler: Bundler, config: DataFlowConfig, source: DataFlowSource) {
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
      if (from === to) {
        return 'A node cannot be connected to itself';
      }

      if (edges.find((e) => e.from === from && e.to === to)) {
        return 'This edge already exists';
      }

      let toNode = nodes[nodeIdToIndex.get(to)];
      if (toNode?.config.func.type === 'trigger') {
        return 'A trigger cannot have any inputs';
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

  function addNode(data: DataFlowManagerData, box: Box, name: string, func: DataFlowNodeFunction) {
    let maxId = Math.max(...data.nodes.map((n) => n.meta.id), 0);

    let nodes: DataFlowManagerNode[] = [
      ...data.nodes,
      {
        config: {
          allow_null_inputs: true,
          name,
          func,
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
  }

  function createNodeName(
    data: DataFlowManagerData,
    prefix: string,
    initialSuffix: number | null = null
  ) {
    let index = initialSuffix;
    let name = typeof index === 'number' ? `${prefix}${index}` : prefix;
    while (data.nodes.some((n) => n.config.name === name)) {
      index = (index ?? 0) + 1;
      name = `${prefix}${index}`;
    }
    return name;
  }

  function addJsNode(box: Box) {
    update((data) => {
      let newNodeName = createNodeName(data, 'node', data.nodes.length);
      return addNode(data, box, newNodeName, { type: 'js', func: `__${newNodeName}` });
    });
  }

  function deleteNodeInternal(data: DataFlowManagerData, index: number) {
    let nodeId = data.nodes[index].meta.id;
    let edges = data.edges.filter((e) => e.from !== nodeId && e.to !== nodeId);
    let nodes = data.nodes.filter((n) => n.meta.id !== nodeId);

    return {
      ...data,
      edges,
      nodes,
    };
  }

  function deleteNode(id: number) {
    update((data) => {
      let index = data.nodeIdToIndex.get(id);
      if (typeof index !== 'number') {
        return data;
      }

      return deleteNodeInternal(data, index);
    });
  }

  return {
    subscribe: store.subscribe,
    set: store.set,
    update,
    compile(): Promise<{ compiled: DataFlowConfig; source: DataFlowSource }> {
      let data = getStore(store);
      return compileCode(bundler, data);
    },
    syncTriggers(triggers: Record<string, TaskTrigger>) {
      update((data) => {
        let idsToRemove: number[] = [];
        let taskTriggerIds = new Set(Object.values(triggers).map((t) => t.task_trigger_id));

        // Remove any trigger nodes that reference nonexistent triggers.
        for (let node of data.nodes) {
          if (node.config.func.type !== 'trigger') {
            continue;
          }

          if (!taskTriggerIds.has(node.config.func.task_trigger_id)) {
            idsToRemove.push(node.meta.id);
          }
        }

        if (idsToRemove.length) {
          data = {
            ...data,
            nodes: data.nodes.filter((n) => !idsToRemove.includes(n.meta.id)),
            edges: data.edges.filter(
              (e) => !idsToRemove.includes(e.from) && !idsToRemove.includes(e.to)
            ),
          };
        }

        // Add nodes for triggers that don't have them yet.
        let newBoxOrigin = 60;
        for (let [triggerLocalId, trigger] of Object.entries(triggers)) {
          let triggerId = trigger.task_trigger_id;
          let node = data.nodes.find(
            (n) => n.config.func.type === 'trigger' && n.config.func.task_trigger_id === triggerId
          );

          if (!node) {
            let nodeName = createNodeName(data, triggerLocalId, null);
            // TODO Find some reasonably appropriate box, accounting for other existing boxes.
            data = addNode(data, { x: newBoxOrigin, y: newBoxOrigin, w: 150, h: 150 }, nodeName, {
              type: 'trigger',
              task_trigger_id: triggerId,
            });
            newBoxOrigin += 40;
          }
        }

        return data;
      });
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
    addJsNode,
    deleteNode,
  };
}
