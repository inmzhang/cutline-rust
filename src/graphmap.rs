use crate::config::TopologyConfig;
use anyhow::{bail, Ok, Result};
use indexmap::IndexMap;
use itertools::Itertools;
use petgraph::{algo::connected_components, graphmap::UnGraphMap};

type CutGraph = UnGraphMap<(u32, u32), bool>;
type Point = (u32, u32);

#[derive(Debug)]
pub struct SearchGraph {
    pub primal: CutGraph,
    pub dual: CutGraph,
    pub unused_qubits: Vec<Point>,
    pub dual_boundaries: Vec<Point>,
}

impl SearchGraph {
    pub fn from_config(config: TopologyConfig) -> Result<Self> {
        let (primal, unused_qubits) = create_primal(&config)?;
        let mut dual = create_dual(&primal);
        let mut dual_boundaries = get_dual_boundary(&dual, config.grid_width, config.grid_height);
        let removed_nodes = remove_dangling_nodes(&mut dual);
        dual_boundaries.retain(|n| !removed_nodes.contains(n));  
        Ok(Self {
            primal,
            unused_qubits,
            dual,
            dual_boundaries,
        })
    }
}

fn create_primal(config: &TopologyConfig) -> Result<(CutGraph, Vec<Point>)> {
    let width = config.grid_width;
    let height = config.grid_height;
    let unused_qubits = &config.unused_qubits;
    let unused_couplers = &config.unused_couplers;
    let mut primal = UnGraphMap::new();
    let qubits_map: IndexMap<_, _> = (0..height)
        .cartesian_product(0..width)
        .filter(|&(y, x)| in_primal(x, y, config.qubit_at_origin))
        .enumerate()
        .map(|(i, (y, x))| ((x, y), i as u32))
        .collect();

    qubits_map.iter().for_each(|(&(x, y), _)| {
        if y == height - 1 {
            return;
        }
        if x > 0 {
            primal.add_edge((x, y), (x - 1, y + 1), true);
        }
        if x < width - 1 {
            primal.add_edge((x, y), (x + 1, y + 1), true);
        }
    });
    // set unused couplers
    primal.all_edges_mut().for_each(|(n1, n2, edge)| {
        let i1 = qubits_map[&n1];
        let i2 = qubits_map[&n2];
        if unused_qubits.contains(&i1)
            || unused_qubits.contains(&i2)
            || unused_couplers.contains(&(i1, i2))
            || unused_couplers.contains(&(i2, i1))
        {
            *edge = false;
        }
    });

    let unused_qubits = qubits_map
        .iter()
        .filter(|&(_, i)| unused_qubits.contains(i))
        .map(|(&p, _)| p)
        .collect_vec();

    // Verify the graph is single connected
    verify_single_connected(&primal, &unused_qubits)?;
    Ok((primal, unused_qubits))
}

fn verify_single_connected(graph: &CutGraph, unused_qubits: &Vec<Point>) -> Result<()> {
    let mut verify_graph = graph.clone();
    for unused_qubit in unused_qubits {
        verify_graph.remove_node(*unused_qubit);
    }
    for (n1, n2, &edge) in graph.all_edges() {
        if !edge {
            verify_graph.remove_edge(n1, n2);
        }
    }
    if connected_components(&verify_graph) != 1 {
        bail!("The graph is not single connected")
    }
    Ok(())
}

fn create_dual(primal: &CutGraph) -> CutGraph {
    let mut dual_graph = UnGraphMap::new();
    for (q1, q2, &used) in primal.all_edges() {
        let router1 = (q1.0, q2.1);
        let router2 = (q2.0, q1.1);
        dual_graph.add_edge(router1, router2, used);
    }
    dual_graph
}

pub fn get_dual_boundary(graph: &CutGraph, grid_width: u32, grid_height: u32) -> Vec<Point> {
    // initial boundary
    let initial_boundaries = graph
        .nodes()
        .filter(|&node| {
            node.0 == 0 || node.0 == grid_width - 1 || node.1 == 0 || node.1 == grid_height - 1
        })
        .collect_vec();

    let mut boundaries = Vec::new();

    // Contract or spread boundary through virtual routes
    for node in initial_boundaries {
        try_set_boundary(node, graph, &mut boundaries);
    }

    boundaries
}

fn try_set_boundary(point: Point, graph: &CutGraph, current_boundaries: &mut Vec<Point>) {
    // If the node is already boundary, then its neighbor should
    // already be explored and set correspondingly.
    if current_boundaries.contains(&point) {
        return;
    }
    current_boundaries.push(point);
    let neighbors = graph.neighbors(point).collect::<Vec<_>>();
    for neighbor in neighbors {
        if *graph.edge_weight(point, neighbor).unwrap() {
            continue;
        }
        try_set_boundary(neighbor, graph, current_boundaries);
    }
}

fn remove_dangling_nodes(graph: &mut CutGraph) -> Vec<Point> {
    let mut remove_nodes = graph.nodes().collect_vec();
    remove_nodes.retain(|&n| graph.edges(n).all(|(_, _, &used)| !used));
    remove_nodes.iter().for_each(|&n| {
        graph.remove_node(n);
    });
    remove_nodes
}

fn in_primal(x: u32, y: u32, start_at_origin: bool) -> bool {
    if y & 1 == 0 {
        if start_at_origin {
            x & 1 == 0
        } else {
            x & 1 == 1
        }
    } else if start_at_origin {
        x & 1 == 1
    } else {
        x & 1 == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_graph_basic() {
        let mut config = TopologyConfig::default();
        config.unused_qubits.push(1);
        let graph = SearchGraph::from_config(config).unwrap();
        assert_eq!(graph.primal.node_count(), 66);
        assert_eq!(graph.primal.edge_count(), 110);
        assert_eq!(graph.dual.node_count(), 66);
        assert_eq!(graph.dual.edge_count(), 110);
    }

    #[test]
    fn test_more_than_one_cc() {
        let mut config = TopologyConfig::default();
        config.unused_qubits.push(11);
        let graph = SearchGraph::from_config(config);
        assert!(graph.is_err());

        let mut config = TopologyConfig::default();
        config.unused_couplers.extend([(11, 17), (23, 17)]);
        let graph = SearchGraph::from_config(config);
        assert!(graph.is_err());
    }

    #[test]
    fn test_dual_boundary() {
        let mut config = TopologyConfig::default();
        config.unused_qubits.extend([5, 11]);
        let graph = SearchGraph::from_config(config).unwrap();
        let dual_graph = &graph.dual;
        let boundaries = &graph.dual_boundaries;
        assert_eq!(dual_graph.node_count(), 66 - 2);
        assert_eq!(dual_graph.edge_count(), 110 - 3);
        assert_eq!(boundaries.len(), 21);
        assert!(boundaries.contains(&(9, 1)));
        assert!(boundaries.contains(&(10, 2)));
    }

    #[test]
    fn test_middle_dangling() {
        let mut config = TopologyConfig::default();
        config.unused_qubits.extend([33, 34]);
        let graph = SearchGraph::from_config(config).unwrap();
        let dual_graph = &graph.dual;
        assert_eq!(dual_graph.node_count(), 66 - 1);
        assert_eq!(dual_graph.edge_count(), 110 - 4)
    }
}
