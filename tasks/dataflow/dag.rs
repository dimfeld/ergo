use bit_set::BitSet;

use super::{DataFlowConfig, DataFlowEdge};
use crate::{Error, Result};

#[derive(Debug)]
pub struct NodeWalker<'a> {
    config: &'a DataFlowConfig,
    current_index: usize,
    active: BitSet,
}

impl<'a> NodeWalker<'a> {
    pub fn starting_from(config: &DataFlowConfig, node_id: u32) -> Result<NodeWalker> {
        let start_idx = config
            .toposorted
            .iter()
            .position(|&n| n == node_id)
            .ok_or(Error::MissingDataFlowNode(node_id))?;

        let max_node = config.nodes.len();

        let mut active = BitSet::with_capacity(max_node + 1);
        Self::find_active_nodes(config, node_id, &mut active)?;

        Ok(NodeWalker {
            config,
            current_index: start_idx,
            active,
        })
    }

    fn find_active_nodes(
        config: &DataFlowConfig,
        start_idx: u32,
        active: &mut BitSet,
    ) -> Result<()> {
        let edges = config
            .edges
            .iter()
            .skip_while(|e| e.from != start_idx)
            .take_while(|e| e.from == start_idx);

        for edge in edges {
            if edge.to >= config.nodes.len() as u32 {
                return Err(Error::BadEdgeIndex(start_idx, edge.to));
            }

            if !active.contains(edge.to as usize) {
                Self::find_active_nodes(config, edge.to, active)?;
            }
        }

        active.insert(start_idx as usize);

        Ok(())
    }
}

impl<'a> Iterator for NodeWalker<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.config.toposorted.len() {
            return None;
        }

        let index = loop {
            let lookup_index = self.current_index;
            self.current_index += 1;

            match self.config.toposorted.get(lookup_index) {
                Some(&node) => {
                    if self.active.contains(node as usize) {
                        break node as usize;
                    }
                }
                None => return None,
            };
        };

        Some(index)
    }
}

pub fn toposort_nodes(num_nodes: usize, edges: &[DataFlowEdge]) -> Result<Vec<u32>> {
    let mut graph = petgraph::graph::DiGraph::<u32, ()>::with_capacity(num_nodes, edges.len());

    let mut node_graph_index = Vec::with_capacity(num_nodes);
    for i in 0..num_nodes {
        let graph_idx = graph.add_node(i as u32);
        node_graph_index.push(graph_idx);
    }

    for &DataFlowEdge { from, to, .. } in edges {
        if from >= num_nodes as u32 {
            return Err(Error::BadEdgeIndex(from, to));
        }

        if to >= num_nodes as u32 {
            return Err(Error::BadEdgeIndex(from, to));
        }

        graph.add_edge(
            node_graph_index[from as usize],
            node_graph_index[to as usize],
            (),
        );
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

#[cfg(test)]
mod tests {
    use crate::{
        dataflow::{
            dag::toposort_nodes,
            node::{DataFlowNodeFunction, DataFlowText},
            DataFlowEdge, DataFlowNode,
        },
        Error,
    };

    fn blank_node() -> DataFlowNode {
        DataFlowNode {
            name: "test_node".into(),
            allow_null_inputs: true,
            func: DataFlowNodeFunction::Text(DataFlowText {
                body: String::new(),
                render_as: crate::dataflow::node::TextRenderAs::PlainText,
            }),
        }
    }

    fn test_edge(from: u32, to: u32) -> DataFlowEdge {
        DataFlowEdge { from, to }
    }

    mod toposort {
        use super::*;

        #[test]
        fn empty() {
            let sorted = toposort_nodes(0, &[]).unwrap();
            assert_eq!(sorted, Vec::<u32>::new());
        }

        #[test]
        fn working() {
            let edges = vec![
                test_edge(0, 1),
                test_edge(0, 2),
                test_edge(0, 3),
                test_edge(2, 1),
                test_edge(2, 4),
                test_edge(3, 1),
                test_edge(3, 2),
                test_edge(3, 4),
                test_edge(6, 2),
            ];

            let sorted = toposort_nodes(7, &edges).unwrap();

            let positions = (0..7)
                .map(|i| {
                    sorted
                        .iter()
                        .position(|&n| n == i)
                        .expect("Finding position for node {i}")
                })
                .collect::<Vec<_>>();

            for edge in &edges {
                assert!(
                    positions[edge.from as usize] < positions[edge.to as usize],
                    "Edge {} comes before its child {}",
                    edge.from,
                    edge.to
                );
            }
        }

        #[test]
        fn errors_on_cycle() {
            let edges = vec![
                test_edge(0, 1),
                test_edge(1, 2),
                test_edge(1, 4),
                test_edge(2, 3),
                test_edge(2, 4),
                test_edge(3, 1),
                test_edge(3, 4),
                test_edge(4, 5),
            ];

            let sort_result = toposort_nodes(6, &edges);
            assert!(matches!(sort_result, Err(Error::DataflowCycle(_))));
        }

        #[test]
        fn errors_on_self_cycle() {
            let edges = vec![
                test_edge(0, 1),
                test_edge(1, 2),
                test_edge(1, 4),
                test_edge(2, 3),
                test_edge(3, 3),
                test_edge(3, 4),
            ];

            let sort_result = toposort_nodes(5, &edges);
            assert!(matches!(sort_result, Err(Error::DataflowCycle(3))));
        }

        #[test]
        fn to_edge_bounds_error() {
            let edges = vec![test_edge(0, 1), test_edge(1, 2), test_edge(1, 3)];

            let sort_result = toposort_nodes(3, &edges);
            assert!(matches!(sort_result, Err(Error::BadEdgeIndex(1, 3))));
        }

        #[test]
        fn from_edge_bounds_error() {
            let edges = vec![test_edge(0, 1), test_edge(1, 2), test_edge(3, 2)];

            let sort_result = toposort_nodes(3, &edges);
            assert!(matches!(sort_result, Err(Error::BadEdgeIndex(3, 2))));
        }
    }

    mod dag_iterator {
        use fxhash::FxHashSet;

        use crate::dataflow::{config::DataFlowConfig, dag::NodeWalker};

        use super::*;

        fn make_config() -> DataFlowConfig {
            let edges = vec![
                test_edge(0, 1),
                test_edge(0, 2),
                test_edge(0, 3),
                test_edge(2, 1),
                test_edge(2, 4),
                test_edge(3, 1),
                test_edge(3, 2),
                test_edge(3, 4),
                test_edge(6, 2),
            ];

            let toposorted = toposort_nodes(7, &edges).unwrap();

            DataFlowConfig {
                nodes: (0..7).map(|_| blank_node()).collect(),
                edges,
                compiled: String::new(),
                map: None,
                toposorted,
            }
        }

        #[test]
        fn from_root() {
            let config = make_config();
            let iter = NodeWalker::starting_from(&config, 0).expect("Creating walker");

            let full_chain = iter.collect::<Vec<_>>();
            println!("Full chain: {:?}", full_chain);

            let mut seen = FxHashSet::default();
            for node in full_chain {
                let after = config.toposorted.iter().skip_while(|&&n| n != node as u32);

                for &n in after {
                    assert!(
                        !seen.contains(&n),
                        "Node {} was seen before node {}",
                        n,
                        node
                    );
                }

                seen.insert(node as u32);
            }

            for i in 0..5 {
                assert!(seen.contains(&i), "should see {i}");
            }
            assert!(!seen.contains(&6), "should not see 6");
        }

        #[test]
        fn from_alternate_root() {
            let config = make_config();
            let iter = NodeWalker::starting_from(&config, 6).expect("Creating walker");

            let full_chain = iter.collect::<Vec<_>>();
            println!("Full chain: {:?}", full_chain);

            let mut seen = FxHashSet::default();
            for node in full_chain {
                let after = config.toposorted.iter().skip_while(|&&n| n != node as u32);

                for &n in after {
                    assert!(
                        !seen.contains(&n),
                        "Node {} was seen before node {}",
                        n,
                        node
                    );
                }

                seen.insert(node as u32);
            }

            assert!(seen.contains(&6), "should see 6");
            assert!(seen.contains(&2), "should see 2");
            assert!(seen.contains(&4), "should see 4");
            assert!(seen.contains(&1), "should see 1");

            assert!(!seen.contains(&0), "should not see 0");
            assert!(!seen.contains(&3), "should not see 3");
        }

        #[test]
        fn from_middle() {
            let config = make_config();
            let iter = NodeWalker::starting_from(&config, 2).expect("Creating walker");

            let full_chain = iter.collect::<Vec<_>>();
            println!("Full chain: {:?}", full_chain);

            let mut seen = FxHashSet::default();
            for node in full_chain {
                let after = config.toposorted.iter().skip_while(|&&n| n != node as u32);

                for &n in after {
                    assert!(
                        !seen.contains(&n),
                        "Node {} was seen before node {}",
                        n,
                        node
                    );
                }

                seen.insert(node as u32);
            }

            assert!(seen.contains(&2), "should see 2");
            assert!(seen.contains(&4), "should see 4");
            assert!(seen.contains(&1), "should see 1");

            assert!(!seen.contains(&0), "should not see 0");
            assert!(!seen.contains(&3), "should not see 3");
            assert!(!seen.contains(&6), "should not see 6");
        }

        #[test]
        fn from_leaf() {
            let config = make_config();
            let iter = NodeWalker::starting_from(&config, 4).expect("Creating walker");

            let full_chain = iter.collect::<Vec<_>>();
            println!("Full chain: {:?}", full_chain);

            assert_eq!(full_chain, vec![4]);
        }

        #[test]
        fn from_past_end() {
            let config = make_config();
            NodeWalker::starting_from(&config, 7).expect_err("Should see error");
        }
    }
}
