use crate::config::TopologyConfig;
use anyhow::{bail, Ok, Result};
use indexmap::IndexMap;
use itertools::Itertools;
use petgraph::{algo::connected_components, graphmap::UnGraphMap};

pub type CutGraph = UnGraphMap<(i32, i32), bool>;
pub type Point = (i32, i32);

#[derive(Debug, Clone)]
pub struct SearchGraph {
    pub config: TopologyConfig,
    pub primal: CutGraph,
    pub dual: CutGraph,
    pub unused_qubits: Vec<Point>,
    pub dual_boundaries: Vec<Point>,
}

impl SearchGraph {
    pub fn from_config(config: TopologyConfig) -> Result<Self> {
        let (primal, unused_qubits) = create_primal(&config)?;
        let mut dual = create_dual(&primal);
        let width = config.width;
        let height = config.height;
        let mut dual_boundaries = get_dual_boundary(&dual, width, height);
        let dangling_nodes = dangling_nodes(&dual);
        dangling_nodes
            .iter()
            .filter(|&n| dual_boundaries.contains(n))
            .for_each(|n| {
                dual.remove_node(*n);
            });
        dual_boundaries.retain(|n| !dangling_nodes.contains(n));
        Ok(Self {
            config,
            primal,
            unused_qubits,
            dual,
            dual_boundaries,
        })
    }

    #[allow(unused)]
    pub fn num_slash(&self) -> usize {
        let primal = &self.primal;
        let primal_width = self.config.width as i32;
        primal
            .nodes()
            .filter(|&n| {
                (n.1 == 0 || n.0 == primal_width - 1)
                    && primal.edge_weight(n, (n.0 - 1, n.1 + 1)).is_some()
            })
            .count()
    }

    #[allow(unused)]
    pub fn num_back_slash(&self) -> usize {
        let primal = &self.primal;
        let primal_width = self.config.width as i32;
        let primal_height = self.config.height as i32;
        primal
            .nodes()
            .filter(|&n| {
                (n.1 == primal_height - 1 || n.0 == primal_width - 1)
                    && primal.edge_weight(n, (n.0 - 1, n.1 - 1)).is_some()
            })
            .count()
    }

    #[inline(always)]
    pub fn edge_index(&self, n1: Point, n2: Point) -> usize {
        ((n1.1 + n2.1) / 2) as usize * (self.config.width - 1) as usize
            + ((n1.0 + n2.0) / 2) as usize
    }

    #[inline(always)]
    pub fn get_edge(&self, index: usize) -> (Point, Point) {
        let width = self.config.width as usize - 1;
        let (quotient, remainder) = ((index / width) as i32, (index % width) as i32);
        if self.primal.contains_node((remainder, quotient)) {
            ((remainder, quotient), (remainder + 1, quotient + 1))
        } else {
            ((remainder, quotient + 1), (remainder + 1, quotient))
        }
    }
}

impl Default for SearchGraph {
    fn default() -> Self {
        let config = TopologyConfig::default();
        Self::from_config(config).unwrap()
    }
}

#[inline(always)]
pub fn duality_map(p1: Point, p2: Point) -> (Point, Point) {
    let dual_p1 = (p1.0, p2.1);
    let dual_p2 = (p2.0, p1.1);
    (dual_p1, dual_p2)
}

fn create_primal(config: &TopologyConfig) -> Result<(CutGraph, Vec<Point>)> {
    let width = config.width;
    let height = config.height;
    let unused_qubits = &config.unused_qubits;
    let unused_couplers = &config.unused_couplers;
    let mut primal = UnGraphMap::new();
    let qubits_map: IndexMap<_, _> = (0..height)
        .cartesian_product(0..width)
        .filter(|&(y, x)| in_primal(x as i32, y as i32, config.qubit_at_origin))
        .enumerate()
        .map(|(i, (y, x))| ((x as i32, y as i32), i as u32))
        .collect();

    qubits_map.iter().for_each(|(&(x, y), _)| {
        if y == (height - 1) as i32 {
            return;
        }
        if x > 0 {
            primal.add_edge((x, y), (x - 1, y + 1), true);
        }
        if x < (width - 1) as i32 {
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
        let (router1, router2) = duality_map(q1, q2);
        dual_graph.add_edge(router1, router2, used);
    }
    dual_graph
}

pub fn get_dual_boundary(graph: &CutGraph, grid_width: u32, grid_height: u32) -> Vec<Point> {
    // initial boundary
    let initial_boundaries = graph
        .nodes()
        .filter(|&node| {
            node.0 == 0
                || node.0 == grid_width as i32 - 1
                || node.1 == 0
                || node.1 == grid_height as i32 - 1
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

fn dangling_nodes(graph: &CutGraph) -> Vec<Point> {
    let mut dangling_nodes = graph.nodes().collect_vec();
    dangling_nodes.retain(|&n| graph.edges(n).all(|(_, _, &used)| !used));
    dangling_nodes
}

fn in_primal(x: i32, y: i32, start_at_origin: bool) -> bool {
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
        assert_eq!(dual_graph.node_count(), 66);
        assert_eq!(
            dual_graph.all_edges().filter(|(_, _, e)| **e).count(),
            110 - 8
        );
    }

    #[test]
    fn test_num_slash() {
        let graph = SearchGraph::default();
        assert_eq!(graph.num_slash(), 10);
        assert_eq!(graph.num_back_slash(), 10);
    }

    #[test]
    fn test_edge_index() {
        let graph = SearchGraph::default();
        assert_eq!(graph.edge_index((0, 1), (1, 0)), 0);
        assert_eq!(graph.edge_index((3, 2), (2, 1)), 13);
        assert_eq!(graph.edge_index((10, 9), (11, 10)), 109);
        assert_eq!(graph.get_edge(0), ((0, 1), (1, 0)));
        assert_eq!(graph.get_edge(13), ((2, 1), (3, 2)));
        assert_eq!(graph.get_edge(109), ((10, 9), (11, 10)));
    }
}
