use crate::{
    config::AlgorithmConfig,
    graph::{duality_map, CutGraph, Point, SearchGraph},
};
use itertools::Itertools;
use petgraph::visit::{Dfs, EdgeFiltered, EdgeRef};
use std::iter::from_fn;

pub type Path = Vec<Point>;
pub type Edge = (Point, Point);
type Split = Vec<Edge>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Cutline {
    pub split: Vec<Edge>,
    pub unbalance: usize,
}

#[derive(Debug, Clone)]
pub struct CutlineWrapped {
    pub split: Vec<usize>,
    pub unbalance: usize,
    pub wedge_candidates: Vec<(usize, usize)>,
    pub dcd_candidates: Vec<(usize, usize)>,
}

impl Cutline {
    pub fn into_wrapped(self, graph: &SearchGraph) -> CutlineWrapped {
        let primal = &graph.primal;
        let split = self
            .split
            .into_iter()
            .filter(|e| primal.edge_weight(e.0, e.1).unwrap().to_owned())
            .collect_vec();
        let wedge_candidates = split
            .iter()
            .combinations(2)
            .filter_map(|comb| {
                let (e1, e2) = (*comb[0], *comb[1]);
                if e1.0 == e2.0 || e1.0 == e2.1 || e1.1 == e2.0 || e1.1 == e2.1 {
                    Some((graph.edge_index(e1.0, e1.1), graph.edge_index(e2.0, e2.1)))
                } else {
                    None
                }
            })
            .collect_vec();
        let dcd_candidates = split
            .iter()
            .filter_map(|&(n1, n2)| {
                let incident_node1 = (2 * n1.0 - n2.0, 2 * n1.1 - n2.1);
                let incident_node2 = (2 * n2.0 - n1.0, 2 * n2.1 - n1.1);
                match (
                    primal.edge_weight(n1, incident_node1).copied(),
                    primal.edge_weight(n2, incident_node2).copied(),
                ) {
                    (Some(true), Some(false)) | (Some(true), None) => Some((
                        graph.edge_index(n1, n2),
                        graph.edge_index(incident_node1, n1),
                    )),
                    (Some(false), Some(true)) | (None, Some(true)) => Some((
                        graph.edge_index(n1, n2),
                        graph.edge_index(n2, incident_node2),
                    )),
                    _ => None,
                }
            })
            .collect_vec();

        let split = split
            .into_iter()
            .map(|(n1, n2)| graph.edge_index(n1, n2))
            .collect_vec();
        CutlineWrapped {
            split,
            unbalance: self.unbalance,
            wedge_candidates,
            dcd_candidates,
        }
    }

    pub fn from_wrapper(wrapper: CutlineWrapped, graph: &SearchGraph) -> Self {
        let split = wrapper
            .split
            .into_iter()
            .map(|e| graph.get_edge(e))
            .collect_vec();
        Cutline {
            split,
            unbalance: wrapper.unbalance,
        }
    }
}

fn path_to_split(path: Path) -> Split {
    path.iter()
        .tuple_windows()
        .map(|(&n1, &n2)| {
            let (n1, n2) = duality_map(n1, n2);
            (n1.min(n2), n1.max(n2))
        })
        .collect_vec()
}

pub fn search_cutlines(graph: &SearchGraph, algorithm_config: &AlgorithmConfig) -> Vec<Cutline> {
    let splits = search_splits(graph, algorithm_config);
    // println!("Found {} splits in total", splits.len());
    let splits = dedup_virtual_dispatch(graph, splits);
    debug_assert!(splits.iter().unique().count() == splits.len());
    // println!("Found {} unique splits after deduplication", splits.len());
    let unused_qubits = &graph.unused_qubits;
    let mut used_qubits = graph.primal.nodes().collect_vec();
    used_qubits.retain(|q| !unused_qubits.contains(q));
    limit_unbalance(graph, splits, algorithm_config.max_unbalance, &used_qubits)
        .into_iter()
        .map_into()
        .collect()
}

fn dedup_virtual_dispatch(graph: &SearchGraph, splits: Vec<Split>) -> Vec<Split> {
    let primal = &graph.primal;
    splits
        .into_iter()
        .unique_by(|split| {
            let mut split = split.clone();
            split.retain(|e| primal.edge_weight(e.0, e.1).unwrap().to_owned());
            split
        })
        .collect_vec()
}

fn limit_unbalance(
    graph: &SearchGraph,
    splits: Vec<Split>,
    max_unbalance: usize,
    used_qubits: &Vec<Point>,
) -> Vec<Cutline> {
    splits
        .into_iter()
        .filter_map(|split| {
            let unbalance = compute_unbalance(graph, used_qubits, &split);
            if unbalance > max_unbalance {
                None
            } else {
                Some(Cutline { split, unbalance })
            }
        })
        .collect()
}

fn search_splits(graph: &SearchGraph, algorithm_config: &AlgorithmConfig) -> Vec<Split> {
    let boundaries = graph.dual_boundaries.clone();
    (0..boundaries.len() - 1)
        .flat_map(|i| {
            let from = boundaries[i];
            let tos = boundaries[i + 1..].to_owned();
            search_paths_between(
                graph,
                from,
                tos,
                algorithm_config.min_search_depth,
                algorithm_config.max_search_depth,
            )
            .map(path_to_split)
            .collect_vec()
        })
        .collect()
}

fn search_paths_between(
    graph: &SearchGraph,
    from: Point,
    tos: Vec<Point>,
    min_path_length: usize,
    max_path_length: usize,
) -> impl Iterator<Item = Path> + '_ {
    let boundaries = &graph.dual_boundaries;
    let graph = &graph.dual;
    // list of visited nodes
    let mut visited: Vec<Point> = vec![from];
    // list of childs of currently exploring path nodes,
    // last elem is list of childs of last visited node
    let mut stack = vec![graph.neighbors(from)];

    from_fn(move || {
        while let Some(children) = stack.last_mut() {
            if let Some(child) = children.next() {
                let depth = compute_depth(graph, &visited);
                if depth + 1 < max_path_length {
                    if tos.contains(&child) {
                        if depth + 1 >= min_path_length {
                            let path = visited.iter().cloned().chain(Some(child)).collect();
                            return Some(path);
                        }
                    } else if !boundaries.contains(&child) && !visited.contains(&child) {
                        visited.push(child);
                        stack.push(graph.neighbors(child));
                    }
                } else {
                    if let Some(c) = Some(child)
                        .into_iter()
                        .chain(children)
                        .find(|c| tos.contains(c))
                    {
                        let path = visited.iter().cloned().chain(Some(c)).collect();
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

fn compute_unbalance(graph: &SearchGraph, used_qubits: &Vec<Point>, split: &Split) -> usize {
    let filtered_graph = EdgeFiltered::from_fn(&graph.primal, |e| {
        let (source, target) = (e.source(), e.target());
        !split.contains(&(source.min(target), source.max(target)))
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
fn compute_depth(graph: &CutGraph, path: &[Point]) -> usize {
    path.iter()
        .tuple_windows()
        .map(|(&n1, &n2)| graph.edge_weight(n1, n2).unwrap().to_owned() as usize)
        .sum()
}
