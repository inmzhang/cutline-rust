use crate::graph::DualGraph;
use itertools::Itertools;
use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    visit::EdgeRef,
};
use smallvec::{smallvec, SmallVec};
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Order {
    None,
    A,
    B,
    C,
    D,
}

impl Order {
    fn all_possibles() -> SmallVec<[Order; 4]> {
        smallvec![Order::A, Order::B, Order::C, Order::D]
    }
}

#[derive(Debug, Clone)]
pub struct Pattern(Vec<Order>);

impl Pattern {
    fn look_up(&self, edge_idx: EdgeIndex) -> Order {
        let idx = edge_idx.index();
        self.0[idx]
    }
}

impl Deref for Pattern {
    type Target = Vec<Order>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Pattern {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Search for all the pattern in the dual graph exhaustively
pub fn search_pattern(dual_graph: &DualGraph) -> Vec<Pattern> {
    let n_edges = dual_graph.edge_count();
    let base_pattern = Pattern(vec![Order::None; n_edges]);
    let mut searched_node = Vec::new();
    search_pattern_rec(dual_graph, base_pattern, &mut searched_node)
}

fn search_pattern_rec(
    dual_graph: &DualGraph,
    base_pattern: Pattern,
    searched_nodes: &mut Vec<NodeIndex>,
) -> Vec<Pattern> {
    let mut patterns = Vec::new();
    for idx in dual_graph.node_indices() {
        if searched_nodes.contains(&idx) {
            continue;
        }
        searched_nodes.push(idx);
        let mut edge_unassigned = SmallVec::<[EdgeIndex; 4]>::new();
        let mut order_assigned = SmallVec::<[Order; 4]>::new();
        for eref in dual_graph.edges(idx) {
            if !eref.weight() {
                continue;
            }
            let eidx = eref.id();
            let order = base_pattern.look_up(eidx);
            if order != Order::None {
                order_assigned.push(order)
            } else {
                edge_unassigned.push(eidx);
            }
        }
        let n_unassigned = edge_unassigned.len();
        if n_unassigned == 0 {
            patterns.push(base_pattern.clone());
            continue;
        }
        let order_unassigned = Order::all_possibles()
            .into_iter()
            .filter(|o| !order_assigned.contains(o));
        debug_assert!(n_unassigned <= order_unassigned.clone().count());
        for order in order_unassigned.permutations(n_unassigned) {
            let mut new_pattern = base_pattern.clone();
            for (order, eidx) in order.iter().zip(edge_unassigned.iter()) {
                new_pattern[eidx.index()] = *order;
            }
            let searched_patterns = search_pattern_rec(dual_graph, new_pattern, searched_nodes);
            patterns.extend(searched_patterns);
        }
    }
    patterns
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use crate::graph::*;

    #[test]
    fn test_pattern_search() {
        let topo = TopologyConfigBuilder::default()
            .grid_width(5)
            .grid_height(4)
            .build()
            .unwrap();
        let config = Config {
            topology: topo,
            ..Default::default()
        };
        let graph = SearchGraph::from_config(config).unwrap();
        let dual_graph = &graph.dual_graph;
        let patterns = search_pattern(dual_graph);
        dbg!(patterns.len());
    }
}
