use crate::graph::PrimalGraph;
use itertools::Itertools;
use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    visit::EdgeRef,
};
use smallvec::SmallVec;
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
};

pub trait Pattern {
    fn look_up(&self, edge_idx: EdgeIndex) -> Order;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Order {
    None,
    A,
    B,
    C,
    D,
}

impl Order {
    fn all_possibles() -> impl Iterator<Item = Order> {
        [Order::A, Order::B, Order::C, Order::D].into_iter()
    }

    fn as_str(&self) -> &'static str {
        match self {
            Order::None => " ",
            Order::A => "A",
            Order::B => "B",
            Order::C => "C",
            Order::D => "D",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExhuastivePattern(Vec<Order>);

impl ExhuastivePattern {
    /// Get the mapping from the coupler coordinate to the order
    pub fn get_pattern_map(&self, primal_graph: &PrimalGraph) -> HashMap<(u32, u32), Order> {
        let mut map = HashMap::new();
        self.0.iter().enumerate().for_each(|(eidx, &ordering)| {
            let (n1, n2) = primal_graph.edge_endpoints(EdgeIndex::new(eidx)).unwrap();
            let q1 = &primal_graph[n1];
            let q2 = &primal_graph[n2];
            let x = (q1.x + q2.x) / 2;
            let y = (q1.y + q2.y) / 2;
            map.insert((x, y), ordering);
        });
        map
    }
}

impl Pattern for ExhuastivePattern {
    fn look_up(&self, edge_idx: EdgeIndex) -> Order {
        let idx = edge_idx.index();
        self.0[idx]
    }
}

impl Display for ExhuastivePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for order in self.0.iter() {
            write!(f, "{}", order.as_str())?;
        }
        Ok(())
    }
}

impl Deref for ExhuastivePattern {
    type Target = Vec<Order>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ExhuastivePattern {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Print the pattern in 2D only for debugging purpose
pub fn print_pattern_in_2d(pattern: &ExhuastivePattern, primal_graph: &PrimalGraph) {
    let map = pattern.get_pattern_map(primal_graph);
    let max_x = map.keys().map(|&(x, _)| x).max().unwrap();
    let max_y = map.keys().map(|&(_, y)| y).max().unwrap();
    let min_x = map.keys().map(|&(x, _)| x).min().unwrap();
    let min_y = map.keys().map(|&(_, y)| y).min().unwrap();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let order = map.get(&(x, y)).unwrap_or(&Order::None);
            print!("{}", order.as_str());
        }
        println!();
    }
}

/// Search for all the pattern in the dual graph exhaustively
pub fn search_pattern_exhaustive(primal_graph: &PrimalGraph) -> Vec<ExhuastivePattern> {
    let n_edges = primal_graph.edge_count();
    let base_pattern = ExhuastivePattern(vec![Order::None; n_edges]);
    let searched_node = Vec::new();
    search_pattern_rec(primal_graph, base_pattern, searched_node)
}

fn search_pattern_rec(
    primal_graph: &PrimalGraph,
    base_pattern: ExhuastivePattern,
    mut searched_nodes: Vec<NodeIndex>,
) -> Vec<ExhuastivePattern> {
    let mut patterns = Vec::new();
    if primal_graph.node_count() == searched_nodes.len() {
        patterns.push(base_pattern);
        return patterns;
    }
    let idx = primal_graph
        .node_indices()
        .filter(|idx| !searched_nodes.contains(idx))
        .take(1)
        .exactly_one()
        .unwrap();
    searched_nodes.push(idx);
    let (order_unassigned, edge_unassigned) =
        unassigned_order_and_edges(idx, primal_graph, &base_pattern);
    let n_unassigned = edge_unassigned.len();
    // if this node has no freedom to color edges,
    // skip to the next one
    if n_unassigned == 0 {
        patterns.extend(search_pattern_rec(
            primal_graph,
            base_pattern,
            searched_nodes,
        ));
        return patterns;
    }
    // recursively search for all the possible patterns
    let allowed_orders = order_unassigned
        .iter()
        .permutations(n_unassigned)
        .filter(|order| {
            edge_unassigned
                .iter()
                .zip(order.iter())
                .all(|(eidx, &order)| {
                    let (n1, n2) = primal_graph.edge_endpoints(*eidx).unwrap();
                    let target_n = if n1 != idx { n1 } else { n2 };
                    let (allowed_orders, _) =
                        unassigned_order_and_edges(target_n, primal_graph, &base_pattern);
                    allowed_orders.contains(order)
                })
        });
    for order in allowed_orders {
        let mut new_pattern = base_pattern.clone();
        for (&o, eidx) in order.iter().zip(edge_unassigned.iter()) {
            new_pattern[eidx.index()] = *o;
        }
        let searched_patterns =
            search_pattern_rec(primal_graph, new_pattern, searched_nodes.clone());
        patterns.extend(searched_patterns);
    }
    patterns
}

fn unassigned_order_and_edges(
    idx: NodeIndex,
    primal_graph: &PrimalGraph,
    base_pattern: &ExhuastivePattern,
) -> (SmallVec<[Order; 4]>, SmallVec<[EdgeIndex; 4]>) {
    let mut assigned_order = SmallVec::<[Order; 4]>::with_capacity(4);
    let mut unassigned_edges = SmallVec::<[EdgeIndex; 4]>::with_capacity(4);
    for eref in primal_graph.edges(idx) {
        if !eref.weight() {
            continue;
        }
        let eidx = eref.id();
        let order = base_pattern.look_up(eidx);
        if order != Order::None {
            assigned_order.push(order)
        } else {
            unassigned_edges.push(eidx);
        }
    }
    (
        Order::all_possibles()
            .filter(move |o| !assigned_order.contains(o))
            .collect(),
        unassigned_edges,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use crate::graph::*;

    #[test]
    fn test_exhuastive_pattern_small_grid() {
        let topo = TopologyConfigBuilder::default()
            .grid_width(4)
            .grid_height(3)
            .build()
            .unwrap();
        let config = Config {
            topology: topo,
            ..Default::default()
        };
        let graph = SearchGraph::from_config(config).unwrap();
        let primal_graph = &graph.primal_graph;
        let patterns = search_pattern_exhaustive(primal_graph);
        // println!("Found {} patterns", patterns.len());
        assert_eq!(patterns.len(), 168)
    }
} 
