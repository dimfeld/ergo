use bit_set::BitSet;
use fxhash::FxHashMap;

use super::{DataFlowConfig, DataFlowNode};
use crate::{Error, Result};

pub struct NodeWalker<'a> {
    config: &'a DataFlowConfig,
    current_index: usize,
    active: BitSet,
}

impl<'a> NodeWalker<'a> {
    pub fn starting_from(config: &DataFlowConfig, node_id: u32) -> Result<NodeWalker> {
        let start_idx = config
            .nodes
            .iter()
            .position(|n| n.id == node_id)
            .ok_or(Error::MissingDataFlowNode(node_id))?;

        let max_node = config
            .nodes
            .iter()
            .map(|n| n.id)
            .max()
            .ok_or(Error::TaskIsEmpty)?;

        let mut active = BitSet::with_capacity(max_node as usize + 1);
        Self::find_active_nodes(config, start_idx, &mut active)?;

        Ok(NodeWalker {
            config,
            current_index: start_idx,
            active,
        })
    }

    fn find_active_nodes(
        config: &DataFlowConfig,
        start_idx: usize,
        active: &mut BitSet,
    ) -> Result<()> {
        let node = &config.nodes[start_idx];
        for &child in &node.dependents {
            if !active.contains(child as usize) {
                let pos = config
                    .nodes
                    .iter()
                    .position(|n| n.id == child)
                    .ok_or(Error::MissingDataFlowDependency(node.id, child))?;
                Self::find_active_nodes(config, pos, active)?;
            }
        }

        active.insert(node.id as usize);
        Ok(())
    }
}

impl<'a> Iterator for NodeWalker<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let index = loop {
            let lookup_index = self.current_index;
            self.current_index += 1;

            match self.config.nodes.get(lookup_index) {
                Some(node) => {
                    if self.active.contains(node.id as usize) {
                        break lookup_index;
                    }
                }
                None => return None,
            };
        };

        Some(index)
    }
}

pub fn toposort_nodes(nodes: &[DataFlowNode]) -> Result<Vec<u32>> {
    let mut graph =
        petgraph::graph::DiGraph::<u32, ()>::with_capacity(nodes.len(), nodes.len() * 3 / 2);
    let mut node_graph_index = FxHashMap::default();
    for node in nodes {
        let graph_idx = graph.add_node(node.id);
        node_graph_index.insert(node.id, graph_idx);
    }

    for node in nodes {
        let graph_parent_idx = node_graph_index[&node.id];
        for child in &node.dependents {
            let graph_child_idx = node_graph_index
                .get(child)
                .ok_or(Error::MissingDataFlowDependency(node.id, *child))?;

            graph.add_edge(graph_parent_idx, *graph_child_idx, ());
        }
    }

    let sorted = petgraph::algo::toposort(&graph, None).map_err(|e| {
        let node_id = graph.node_weight(e.node_id()).copied().unwrap_or(0);
        Error::DataflowCycle(node_id)
    })?;

    let node_ids = sorted
        .into_iter()
        .map(|node_idx| graph.node_weight(node_idx).copied().unwrap())
        .collect::<Vec<_>>();
    Ok(node_ids)
}
