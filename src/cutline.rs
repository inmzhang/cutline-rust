use crate::{
    config::AlgorithmConfig,
    graph::{duality_map, CutGraph, Point, SearchGraph},
};
use indexmap::IndexSet;
use itertools::Itertools;
use petgraph::visit::{Dfs, EdgeFiltered, EdgeRef};
use rayon::prelude::*;
use std::iter::from_fn;

pub type Path = Vec<Point>;

#[derive(Debug, Clone)]
pub struct Cutline {
    pub path: Path,
    pub unbalance: usize,
}

pub fn search_cutlines(graph: &SearchGraph, algorithm_config: &AlgorithmConfig) -> Vec<Cutline> {
    let paths = search_paths(graph, algorithm_config);
    dbg!(paths.len());
    let paths = dedup_virtual_dispatch(graph, paths);
    debug_assert!(paths.iter().unique().count() == paths.len());
    dbg!(paths.len());
    let unused_qubits = &graph.unused_qubits;
    let mut used_qubits = graph.primal.nodes().collect_vec();
    used_qubits.retain(|q| !unused_qubits.contains(q));
    limit_unbalance(graph, paths, algorithm_config.max_unbalance, &used_qubits)
}

fn dedup_virtual_dispatch(graph: &SearchGraph, paths: Vec<Path>) -> Vec<Path> {
    let dual = &graph.dual;
    paths.into_iter().unique_by(|path| {
        let mut qubit_to_remove = Vec::new();
        path.iter()
            .tuple_windows()
            .for_each(|(&n1, &n2)| {
                if !dual.edge_weight(n1, n2).unwrap().to_owned() {
                    qubit_to_remove.extend([n1, n2]);
                }
            });
        let mut unique_path = path.clone();
        unique_path.retain(|q| !qubit_to_remove.contains(q));
        unique_path
    }).collect_vec()
}

fn limit_unbalance(
    graph: &SearchGraph,
    paths: Vec<Path>,
    max_unbalance: usize,
    used_qubits: &Vec<Point>,
) -> Vec<Cutline> {
    paths
        .into_par_iter()
        .filter_map(|path| {
            let unbalance = compute_unbalance(graph, used_qubits, &path);
            if unbalance > max_unbalance {
                None
            } else {
                Some(Cutline { path, unbalance })
            }
        })
        .collect()
}

fn search_paths(graph: &SearchGraph, algorithm_config: &AlgorithmConfig) -> Vec<Path> {
    let boundaries = graph.dual_boundaries.clone();
    boundaries
        .into_iter()
        .permutations(2)
        .map(|v| (v[0], v[1]))
        .collect_vec()
        .into_par_iter()
        .flat_map(|(n1, n2)| {
            search_paths_between(
                graph,
                n1,
                n2,
                algorithm_config.min_search_depth,
                algorithm_config.max_search_depth,
            )
            .collect_vec()
        })
        .collect()
}

/// Search all paths between the two nodes with dfs variant.
fn search_paths_between(
    graph: &SearchGraph,
    from: Point,
    to: Point,
    min_path_length: usize,
    max_path_length: usize,
) -> impl Iterator<Item = Path> + '_ {
    let boundaries = &graph.dual_boundaries;
    let graph = &graph.dual;
    // list of visited nodes
    let mut visited: IndexSet<Point> = IndexSet::from_iter(Some(from));
    // list of childs of currently exploring path nodes,
    // last elem is list of childs of last visited node
    let mut stack = vec![graph.neighbors(from)];

    from_fn(move || {
        while let Some(children) = stack.last_mut() {
            if let Some(child) = children.next() {
                let depth = compute_depth(graph, &visited);
                if depth + 1 < max_path_length {
                    if child == to {
                        if depth + 1 >= min_path_length {
                            let path = visited.iter().cloned().chain(Some(to)).collect();
                            return Some(path);
                        }
                    } else if !boundaries.contains(&child) && !visited.contains(&child) {
                        visited.insert(child);
                        stack.push(graph.neighbors(child));
                    }
                } else {
                    if (child == to || children.any(|v| v == to)) && depth + 1 >= min_path_length {
                        let path = visited.iter().cloned().chain(Some(to)).collect();
                        return Some(path);
                    }
                    stack.pop();
                    visited.pop();
                }
            } else {
                stack.pop();
                visited.pop();
            }
        }
        None
    })
}

fn compute_unbalance(graph: &SearchGraph, used_qubits: &Vec<Point>, path: &Path) -> usize {
    let filtered_edges = path
        .iter()
        .tuple_windows()
        .map(|(&n1, &n2)| {
            let (n1, n2) = duality_map(n1, n2);
            (n1.min(n2), n1.max(n2))
        })
        .collect_vec();
    let filtered_graph = EdgeFiltered::from_fn(&graph.primal, |e| {
        let (source, target) = (e.source(), e.target());
        !filtered_edges.contains(&(source.min(target), source.max(target)))
    });
    let mut dfs = Dfs::new(&filtered_graph, used_qubits[0]);
    let mut count = 0;
    while let Some(qubit) = dfs.next(&filtered_graph) {
        if used_qubits.contains(&qubit) {
            count += 1;
        }
    }
    let count2 = used_qubits.len() - count;
    count.max(count2) - count.min(count2)
}

#[inline(always)]
fn compute_depth(graph: &CutGraph, path: &IndexSet<Point>) -> usize {
    path.iter()
        .tuple_windows()
        .map(|(&n1, &n2)| graph.edge_weight(n1, n2).unwrap().to_owned() as usize)
        .sum()
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::config::*;

//     #[test]
//     fn test_search_paths() {
//         let topo = TopologyConfigBuilder::default()
//             .grid_width(12)
//             .grid_height(11)
//             .unused_qubits(vec![21])
//             .build()
//             .unwrap();
//         let graph = SearchGraph::from_config(topo).unwrap();

//         let algo = AlgorithmConfigBuilder::default()
//             .min_search_depth(0)
//             .max_search_depth(11)
//             .max_unbalance(6)
//             .build()
//             .unwrap();

//         let cutlines = search_cutlines(&graph, &algo);
//         println!("{:?}", cutlines[0]);
//         assert_eq!(cutlines.len(), 5);
//     }
// }
