use crate::config::{AlgorithmConfig, Config, TopologyConfig};
use anyhow::{bail, Ok, Result};
use indexmap::IndexMap;
use petgraph::algo::connected_components;
use petgraph::graph::{NodeIndex, UnGraph};
use petgraph::visit::EdgeRef;

/// Qubit in the primal graph
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
struct Qubit {
    x: u32,
    y: u32,
    qid: u32,
    used: bool,
}

/// Coupler in the primal graph
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct Coupler {
    used: bool,
}

/// Router node in the dual graph
#[derive(Debug, PartialEq, Eq, Hash, Default, Clone, Copy)]
pub struct Router {
    x: u32,
    y: u32,
    boundary: bool,
}

impl Router {
    fn new(x: u32, y: u32) -> Self {
        Router {
            x,
            y,
            boundary: false,
        }
    }
}

/// Edge in the dual graph
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Route {
    /// Dual edge of a used coupler in the primal graph
    Real,
    /// Dual edge of a unused coupler in the primal graph
    Virtual,
}

impl Route {
    fn is_virtual(&self) -> bool {
        match self {
            Route::Real => false,
            Route::Virtual => true,
        }
    }
}

impl From<bool> for Route {
    fn from(value: bool) -> Self {
        if value {
            Route::Real
        } else {
            Route::Virtual
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Direction {
    UR,
    DR,
    UL,
    DL,
}

impl Direction {
    fn apply(&self, before: (u32, u32)) -> (u32, u32) {
        match self {
            Direction::UR => (before.0.wrapping_add(1), before.1.wrapping_add(1)),
            Direction::DR => (before.0.wrapping_add(1), before.1.wrapping_sub(1)),
            Direction::UL => (before.0.wrapping_sub(1), before.1.wrapping_add(1)),
            Direction::DL => (before.0.wrapping_sub(1), before.1.wrapping_sub(1)),
        }
    }
}

type PrimalGraph = UnGraph<Qubit, Coupler, u32>;
type DualGraph = UnGraph<Router, Route, u32>;

/// Search graph in the algorithm
#[derive(Debug)]
pub struct SearchGraph {
    /// The primal graph consists of all qubits and couplers, including unused ones
    primal_graph: PrimalGraph,

    /// The dual graph consists of routers and edges
    dual_graph: DualGraph,

    /// Algorithm configure
    config: AlgorithmConfig,
}

impl SearchGraph {
    pub fn used_qubits(&self) -> u32 {
        self.primal_graph
            .node_indices()
            .filter(|&i| self.primal_graph[i].used)
            .count() as u32
    }

    pub fn used_couplers(&self) -> u32 {
        self.primal_graph
            .edge_indices()
            .filter(|&i| self.primal_graph[i].used)
            .count() as u32
    }

    pub fn from_config(config: Config) -> Result<Self> {
        let qubits = create_qubits(&config.topology);
        let primal_graph = create_primal_graph(&qubits, &config.topology.unused_couplers)?;
        let mut dual_graph = create_dual_graph(&primal_graph);
        set_boundary(
            &mut dual_graph,
            config.topology.grid_width,
            config.topology.grid_height,
        );
        Ok(SearchGraph {
            primal_graph,
            dual_graph,
            config: config.algorithm,
        })
    }

    pub fn dual_boundaries(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.dual_graph
            .node_indices()
            .filter(|&i| self.dual_graph[i].boundary)
    }
}

fn create_qubits(topology: &TopologyConfig) -> IndexMap<(u32, u32), Qubit> {
    let full_grid =
        (0..topology.grid_height).flat_map(|y| (0..topology.grid_width).map(move |x| (x, y)));
    let mut qubits = IndexMap::new();
    // Get all qubits and routers
    for cell in full_grid {
        if is_qubit(cell.0, cell.1, topology.qubit_at_origin) {
            let qid = qubits.len() as u32;
            let used = !topology.unused_qubits.contains(&qid);
            qubits.insert(
                cell,
                Qubit {
                    x: cell.0,
                    y: cell.1,
                    qid,
                    used,
                },
            );
        }
    }
    qubits
}

fn create_primal_graph(
    qubits: &IndexMap<(u32, u32), Qubit>,
    unused_couplers: &[(u32, u32)],
) -> Result<PrimalGraph> {
    let mut primal_graph = UnGraph::default();
    for qubit in qubits.values() {
        primal_graph.add_node(*qubit);
    }
    for (position, qubit) in qubits {
        for direction in [Direction::UL, Direction::UR] {
            let position2 = direction.apply(*position);
            if let Some(qubit2) = qubits.get(&position2) {
                let q1 = qubit.qid;
                let q2 = qubit2.qid;
                let coupler_is_unused = unused_couplers.contains(&(q1, q2))
                    || unused_couplers.contains(&(q2, q1))
                    || !qubit.used
                    || !qubit2.used;
                let coupler = Coupler {
                    used: !coupler_is_unused,
                };
                primal_graph.add_edge(q1.into(), q2.into(), coupler);
            }
        }
    }

    // Verify the graph is single connected
    let mut verify_graph = primal_graph.clone();
    verify_graph.retain_edges(|graph, eidx| {
        let coupler = &graph[eidx];
        coupler.used
    });
    verify_graph.retain_nodes(|graph, nidx| {
        let qubit = &graph[nidx];
        qubit.used
    });
    if connected_components(&verify_graph) > 1 {
        bail!("The primal graph has more than 1 connected components.");
    }
    Ok(primal_graph)
}

fn create_dual_graph(primal_graph: &UnGraph<Qubit, Coupler, u32>) -> DualGraph {
    let mut dual_graph = UnGraph::default();
    let mut routers = IndexMap::new();
    for eidx in primal_graph.edge_indices() {
        let (q1, q2) = primal_graph.edge_endpoints(eidx).unwrap();
        let q1 = primal_graph[q1];
        let q2 = primal_graph[q2];
        let router1 = Router::new(q1.x, q2.y);
        let router2 = Router::new(q2.x, q1.y);
        let n1 = routers
            .entry(router1)
            .or_insert_with(|| dual_graph.add_node(router1))
            .to_owned();
        let n2 = routers
            .entry(router2)
            .or_insert_with(|| dual_graph.add_node(router2))
            .to_owned();
        dual_graph.add_edge(n1, n2, primal_graph[eidx].used.into());
    }
    dual_graph
}

fn set_boundary(dual_graph: &mut DualGraph, grid_width: u32, grid_height: u32) {
    // initial boundary
    let initial_boundaries = dual_graph
        .node_indices()
        .filter(|&idx| {
            let node = dual_graph[idx];
            node.x == 0 || node.x == grid_width - 1 || node.y == 0 || node.y == grid_height - 1
        })
        .collect::<Vec<_>>();

    // Contract or spread boundary through virtual routes
    for idx in initial_boundaries {
        try_set_boundary(idx, dual_graph)
    }
    // Remove all dangling boundary nodes
    dual_graph.retain_nodes(|graph, idx| {
        let node = graph[idx];
        if !node.boundary {
            true
        } else {
            !graph.edges(idx).all(|eref| eref.weight().is_virtual())
        }
    });
}

fn try_set_boundary(idx: NodeIndex, graph: &mut DualGraph) {
    let node = &mut graph[idx];
    // If the node is already boundary, then its neighbor should
    // already be explored and set correspondingly.
    if node.boundary {
        return;
    }
    node.boundary = true;
    let eids = graph.edges(idx).map(|eref| eref.id()).collect::<Vec<_>>();
    for eidx in eids {
        let edge = graph[eidx];
        if !edge.is_virtual() {
            continue;
        }
        let (n1, n2) = graph.edge_endpoints(eidx).unwrap();
        let target = if n1 != idx { n1 } else { n2 };
        try_set_boundary(target, graph)
    }
}

fn is_qubit(x: u32, y: u32, start_at_origin: bool) -> bool {
    if y % 2 == 0 {
        if start_at_origin {
            x % 2 == 0
        } else {
            x % 2 == 1
        }
    } else if start_at_origin {
        x % 2 == 1
    } else {
        x % 2 == 0
    }
}

#[cfg(test)]
mod tests {
    use petgraph::visit::NodeIndexable;

    use super::*;

    macro_rules! find_node_at_position {
        ($graph:ident, $x:expr, $y:expr) => {
            $graph.node_indices().find(|&idx| {
                let node = $graph[idx];
                node.x == $x && node.y == $y
            })
        };
    }

    #[test]
    fn test_search_graph_basic() {
        let mut config = Config::default();
        let num_cell = config.topology.grid_height * config.topology.grid_width;

        config.topology.unused_qubits.push(1);
        let graph = SearchGraph::from_config(config).unwrap();
        assert_eq!(graph.used_qubits(), 66 - 1);
        assert_eq!(graph.used_couplers(), 110 - 2);
        let primal_graph = &graph.primal_graph;
        assert!(!primal_graph[primal_graph.from_index(1)].used);
        assert!(primal_graph[primal_graph.from_index(55)].used);

        let dual_graph = &graph.dual_graph;

        assert_eq!(
            (dual_graph.node_count() + primal_graph.node_count()) as u32,
            num_cell
        );
        assert_eq!(dual_graph.edge_count(), primal_graph.edge_count());
    }

    #[test]
    fn test_more_than_one_cc() {
        let mut config = Config::default();
        config.topology.unused_qubits.push(11);
        let graph = SearchGraph::from_config(config);
        assert!(graph.is_err());

        let mut config = Config::default();
        config.topology.unused_couplers.extend([(11, 17), (23, 17)]);
        let graph = SearchGraph::from_config(config);
        assert!(graph.is_err());
    }

    #[test]
    fn test_dual_boundary() {
        let mut config = Config::default();
        config.topology.unused_qubits.extend([5, 11]);
        let graph = SearchGraph::from_config(config).unwrap();
        let primal_graph = &graph.primal_graph;
        let dual_graph = &graph.dual_graph;
        assert_eq!(primal_graph.node_count(), 66);
        assert_eq!(dual_graph.node_count(), 66 - 2);
        assert_eq!(primal_graph.edge_count(), 110);
        assert_eq!(dual_graph.edge_count(), 110 - 3);
        assert_eq!(graph.dual_boundaries().count(), 21);
        let n1 = find_node_at_position!(dual_graph, 9, 1).unwrap();
        let n2 = find_node_at_position!(dual_graph, 10, 2).unwrap();
        assert!(dual_graph[n1].boundary);
        assert!(dual_graph[n2].boundary);
    }
}
