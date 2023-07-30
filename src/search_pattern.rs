use crate::{
    graph::{Point, SearchGraph},
    pattern::{get_edge_index, slash_index, BitPattern, Context, Order, Pattern, VecPattern},
};
use itertools::Itertools;
use smallvec::SmallVec;
use std::collections::HashSet;

pub fn search_bit_patterns(graph: &SearchGraph) -> impl Iterator<Item = BitPattern> {
    let n_slash = graph.num_slash();
    let n_back_slash = graph.num_back_slash();
    let n_bits = 1 + n_slash + n_back_slash;
    if n_bits >= 32 {
        panic!("Number of patterns is too large! The sum of number of slash and back slash should be less than 32.");
    }
    let max_num: u32 = (1 << n_bits) - 1;
    let dead_indices = dead_slash_indices(graph);
    (0..=max_num)
        .filter(move |n| dead_indices.iter().all(|&i| n & (1 << i) == 0))
        .map(move |n| BitPattern::with_capacity_and_blocks(n_bits, vec![n]))
}

fn dead_slash_indices(graph: &SearchGraph) -> Vec<usize> {
    let mut live_slash = HashSet::new();
    let n_slash = graph.num_slash();
    let n_back_slash = graph.num_back_slash();
    graph.primal.all_edges().for_each(|(n1, n2, &weight)| {
        if !weight {
            return;
        }
        let index = slash_index(
            n1,
            n2,
            graph.config.qubit_at_origin,
            graph.config.height,
            n_slash,
        );
        live_slash.insert(index);
    });
    (1..=(n_slash + n_back_slash))
        .filter(|i| !live_slash.contains(i))
        .collect()
}

#[allow(unused)]
pub fn search_vec_patterns(graph: &SearchGraph) -> Vec<VecPattern> {
    let n_edges = graph.primal.edge_count();
    search_vec_patterns_rec(graph, vec![None; n_edges], HashSet::new())
}

fn search_vec_patterns_rec(
    graph: &SearchGraph,
    base_pattern: VecPattern,
    mut searched_nodes: HashSet<Point>,
) -> Vec<VecPattern> {
    let mut patterns = Vec::new();
    let primal = &graph.primal;
    if primal.node_count() == searched_nodes.len() {
        patterns.push(base_pattern);
        return patterns;
    }
    let next_node = primal
        .nodes()
        .find(|n| !searched_nodes.contains(n))
        .unwrap();
    searched_nodes.insert(next_node);

    let (order_unassigned, neighbors_unassigned) =
        unassigned_order_and_neighbors(next_node, graph, &base_pattern);
    let n_unassigned = neighbors_unassigned.len();
    // if this node has no freedom to color edges,
    // skip to the next one
    if n_unassigned == 0 {
        patterns.extend(search_vec_patterns_rec(graph, base_pattern, searched_nodes));
        return patterns;
    }
    // recursively search for all the possible patterns
    let allowed_orders = order_unassigned
        .into_iter()
        .permutations(n_unassigned)
        .filter(|order| {
            neighbors_unassigned
                .iter()
                .zip(order.iter())
                .all(|(&neighbor, order)| {
                    let (allowed_orders, _) =
                        unassigned_order_and_neighbors(neighbor, graph, &base_pattern);
                    allowed_orders.contains(order)
                })
        });
    for order in allowed_orders {
        let mut new_pattern = base_pattern.clone();
        for (&o, &neighbor) in order.iter().zip(neighbors_unassigned.iter()) {
            let index = get_edge_index(next_node, neighbor, (graph.config.width - 1) as usize);
            new_pattern[index] = Some(o);
        }
        let searched_patterns = search_vec_patterns_rec(graph, new_pattern, searched_nodes.clone());
        patterns.extend(searched_patterns);
    }
    patterns
}

fn unassigned_order_and_neighbors(
    node: Point,
    graph: &SearchGraph,
    base_pattern: &VecPattern,
) -> (SmallVec<[Order; 4]>, SmallVec<[Point; 4]>) {
    let primal = &graph.primal;
    let mut assigned_order = SmallVec::<[Order; 4]>::with_capacity(4);
    let mut unassigned_neighbors = SmallVec::<[Point; 4]>::with_capacity(4);
    let context = Context::from_graph(graph);
    for neighbor in primal.neighbors(node) {
        if !primal.edge_weight(node, neighbor).unwrap() {
            continue;
        }
        if let Some(order) = base_pattern.look_up(node, neighbor, &context) {
            assigned_order.push(order)
        } else {
            unassigned_neighbors.push(neighbor);
        }
    }
    (
        Order::all_possibles()
            .filter(move |o| !assigned_order.contains(o))
            .collect(),
        unassigned_neighbors,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{TopologyConfig, TopologyConfigBuilder};

    macro_rules! test_n_bit_pattern {
        ($unused:expr, $nbits:expr) => {
            let mut config = TopologyConfig::default();
            config.unused_qubits.extend($unused);
            let graph = SearchGraph::from_config(config).unwrap();
            assert_eq!(search_bit_patterns(&graph).count(), 1 << $nbits);
        };
    }

    #[test]
    fn test_bit_pattern_number() {
        test_n_bit_pattern!(Vec::<u32>::new(), 21);
        test_n_bit_pattern!([6], 20);
        test_n_bit_pattern!([54, 60, 4, 5, 11, 17], 19);
        test_n_bit_pattern!([21], 21);
    }

    #[test]
    fn test_vec_pattern() {
        let config = TopologyConfigBuilder::default()
            .width(4)
            .height(3)
            .build()
            .unwrap();
        let graph = SearchGraph::from_config(config).unwrap();
        let patterns = search_vec_patterns(&graph);
        println!("Found {} patterns", patterns.len());
        assert_eq!(patterns.len(), 168)
    }
}
