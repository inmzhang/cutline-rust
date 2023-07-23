use crate::{
    config::AlgorithmConfig,
    graph::{CutGraph, Point, SearchGraph},
};
use indexmap::IndexSet;
use itertools::Itertools;
use rayon::prelude::*;
use std::iter::from_fn;

type Path = Vec<Point>;

#[derive(Debug, Clone)]
pub struct Cutline {
    pub path: Path,
    pub unbalance: usize,
}

pub fn search_cutlines(
    graph: &'static SearchGraph,
    algorithm_config: &AlgorithmConfig,
) -> Vec<Cutline> {
    let paths = search_paths(graph, algorithm_config);
    dbg!(paths.len());
    let unused_qubits = &graph.unused_qubits;
    let mut used_qubits = graph.primal.nodes().collect_vec();
    used_qubits.retain(|q| !unused_qubits.contains(q));
    limit_unbalance(paths, algorithm_config.max_unbalance, &used_qubits)
}

fn limit_unbalance(
    paths: Vec<Path>,
    max_unbalance: usize,
    used_qubits: &Vec<Point>,
) -> Vec<Cutline> {
    paths
        .into_par_iter()
        .filter_map(|path| {
            let unbalance = compute_unbalance(used_qubits, &path);
            if unbalance > max_unbalance {
                None
            } else {
                Some(Cutline { path, unbalance })
            }
        })
        .collect()
}

fn search_paths(graph: &'static SearchGraph, algorithm_config: &AlgorithmConfig) -> Vec<Path> {
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
    graph: &'static SearchGraph,
    from: Point,
    to: Point,
    min_path_length: usize,
    max_path_length: usize,
) -> impl Iterator<Item = Path> {
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
                if depth < max_path_length {
                    if child == to {
                        if depth >= min_path_length {
                            let path = visited.iter().cloned().chain(Some(to)).collect();
                            return Some(path);
                        }
                    } else if !boundaries.contains(&child) && !visited.contains(&child) {
                        visited.insert(child);
                        stack.push(graph.neighbors(child));
                    }
                } else {
                    if (child == to || children.any(|v| v == to)) && depth >= min_path_length {
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

fn compute_unbalance(used_qubits: &Vec<Point>, path: &Path) -> usize {
    let c1 = used_qubits
        .iter()
        .filter(|&&q| component_parity(path, q))
        .count();
    let c2 = used_qubits.len() - c1;
    c1.max(c2) - c1.min(c2)
}

// TODO: Debug
#[inline(always)]
fn component_parity(path: &Path, qubit: Point) -> bool {
    let length = path.len();
    path.iter()
        .enumerate()
        .filter(|&(i, &(nx, ny))| {
            if ny != qubit.1
                || nx > qubit.0
                || (i != 0 && i != length - 1 && path[i - 1].1 == path[i + 1].1)
            {
                return false;
            }
            true
        })
        .count()
        % 2
        == 1
}

#[inline(always)]
fn compute_depth(graph: &'static CutGraph, path: &IndexSet<Point>) -> usize {
    path.iter()
        .tuple_windows()
        .map(|(&n1, &n2)| graph.edge_weight(n1, n2).unwrap().to_owned() as usize)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    #[test]
    fn test_search_paths() {
        let topo = TopologyConfigBuilder::default()
            .grid_width(15)
            .grid_height(14)
            // .unused_qubits(vec![21])
            .build()
            .unwrap();
        let graph = Box::new(SearchGraph::from_config(topo).unwrap());
        let static_ref: &'static SearchGraph = Box::leak(graph);

        let algo = AlgorithmConfigBuilder::default()
            .min_search_depth(5)
            .max_search_depth(11)
            .max_unbalance(20)
            .build()
            .unwrap();

        // let paths = search_paths(static_ref, &algo);
        let cutlines = search_cutlines(static_ref, &algo);
        // assert_eq!(paths.len(), 5);
        println!("{:?}", cutlines[0]);
        assert_eq!(cutlines.len(), 5);
    }
}
