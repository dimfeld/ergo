import type { DataFlowConfig, DataFlowEdge, DataFlowNode } from '$lib/api_types';
import zip from 'just-zip-it';
import { get as getStore, writable } from 'svelte/store';
import type { Box } from './canvas/drag';
import { toposort_nodes } from 'ergo-wasm';
import camelCase from 'just-camel-case';

export interface DataFlowNodeMeta {
  position: Box;
  splitPos: number;
  lastOutput: string;
}

export interface DataFlowSource {
  nodes: DataFlowNodeMeta[];
}

export interface DataFlowManagerData {
  nodes: { config: DataFlowNode; meta: DataFlowNodeMeta }[];
  edges: DataFlowEdge[];
  toposorted: number[];
}

export function dataflowManager(config: DataFlowConfig, source: DataFlowSource) {
  const nodes = zip(config?.nodes || [], source?.nodes || []).map(([config, meta]) => {
    return {
      config,
      meta,
    };
  });

  let edges = config?.edges || [];

  let store = writable({
    nodes,
    edges,
    toposorted: toposort_nodes(nodes.length, edges),
  });

  function update(updateFn: (data: DataFlowManagerData) => DataFlowManagerData) {
    store.update((data) => {
      let result = updateFn(data);
      toposort_nodes(result.nodes.length, result.edges);
      return result;
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

      return {
        compiled: {
          nodes: nodeConfig,
          edges: data.edges,
          toposorted: data.toposorted,
        },
        source: {
          nodes: nodeSource,
        },
      };
    },
    addEdge(from: number, to: number, edgeName?: string) {
      update((data) => {
        let existingEdge = data.edges.find((e) => e.from === from && e.to === to);
        if (existingEdge) {
          return data;
        }

        if (from >= data.nodes.length) {
          throw new Error('from node does not exist');
        }
        if (to >= data.nodes.length) {
          throw new Error('to node does not exist');
        }

        let name = edgeName;
        if (!name) {
          let node = data.nodes[to];
          name = node.config.name;
          if (/[^a-zA-Z0-9]/.test(name)) {
            name = camelCase(name);
          }
        }

        data.edges.push({
          from,
          to,
          name,
        });
        return data;
      });
    },
    deleteEdge(from: number, to: number) {
      update((data) => {
        data.edges = data.edges.filter((e) => e.from !== from || e.to !== to);
        return data;
      });
    },
    addNode(box: Box) {
      update((data) => {
        let newNodeIndex = nodes.length;
        let newNodeName = `node${newNodeIndex}`;
        while (data.nodes.some((n) => n.config.name === newNodeName)) {
          newNodeIndex += 1;
          newNodeName = `node${newNodeIndex}`;
        }

        data.nodes.push({
          config: {
            allow_null_inputs: true,
            name: newNodeName,
            func: {
              type: 'js',
              code: '',
              format: 'Expression',
            },
          },
          meta: {
            position: box,
            splitPos: 75,
            lastOutput: '',
          },
        });

        return data;
      });
    },
    deleteNodeByIndex(index: number) {
      update((data) => {
        data.nodes.splice(index, 1);
        data.edges = data.edges.filter((e) => e.from !== index && e.to !== index);
        return data;
      });
    },
    deleteNode(name: string) {
      update((data) => {
        let index = data.nodes.findIndex((n) => n.config.name === name);
        if (index >= 0) {
          data.nodes.splice(index, 1);
          data.edges = data.edges.filter((e) => e.from !== index && e.to !== index);
        }

        return data;
      });
    },
  };
}
