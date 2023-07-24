use crate::config::AlgorithmConfig;
use crate::cutline::Path;
use crate::graph::{Point, SearchGraph};
use crate::pattern::{Order, Pattern, Context};
use itertools::Itertools;
use rayon::prelude::*;
use std::collections::HashMap;

pub fn max_min_cost<P>(
    graph: &SearchGraph,
    patterns: Vec<P>,
    paths: Vec<Path>,
    algorithm_config: &AlgorithmConfig,
) -> usize
where
    P: Pattern + Send,
{
    patterns
        .into_par_iter()
        .map(|pattern| calculate_min_cost(graph, pattern, &paths, algorithm_config))
        .max()
        .unwrap()
}

fn calculate_min_cost<P>(
    graph: &SearchGraph,
    pattern: P,
    paths: &[Path],
    algorithm_config: &AlgorithmConfig,
) -> usize 
where
    P: Pattern,
{   
    let context = Context::from_graph(graph);
    let order_map = graph.dual.all_edges().map(|(n1, n2, &weight)| {
        let (n1, n2) = (n1.min(n2), n1.max(n2));
        ((n1, n2), pattern.look_up(n1, n2, &context))
    }).collect();
    let mut order_cache = HashMap::new();
    paths
        .iter()
        .map(|path| cost_for_path(&order_map, path, algorithm_config))
        .min()
        .unwrap()
}

fn cost_for_path(
    order_map: &HashMap<(Point, Point), Order>,
    path: &Path,
    algorithm_config: &AlgorithmConfig,
) -> usize {
    let circuit_depth = algorithm_config.circuit_depth;
    let ordering_primitive = &algorithm_config.ordering;
    let ordering = ordering_primitive
        .iter()
        .cycle()
        .take(circuit_depth)
        .cloned()
        .collect_vec();
    // start and end elimination
    todo!()
}
