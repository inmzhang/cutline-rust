use crate::config::{AlgorithmConfig, Config};
use petgraph::graph::UnGraph;
use indexmap::IndexMap;

/// Qubit in the primal graph
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
struct Qubit {
    x: u32,
    y: u32,
    qid: u32,
    used: bool,
}

/// Coupler in the primal graph
#[derive(Debug, PartialEq, Eq, Hash)]
struct Coupler {
    q1: u32,
    q2: u32,
    used: bool,
}

/// Router node in the dual graph
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Router(u32, u32);

/// Edge in the dual graph
#[derive(Debug)]
pub enum RouterEdge {
    /// Dual edge of a working coupler in the primal graph
    Real,
    /// Dual edge of a broken coupler in the primal graph
    Virtual,
}

#[derive(Debug)]
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

/// Search graph in the algorithm
#[derive(Debug)]
pub struct SearchGraph {
    /// The primal graph consists of all qubits and couplers, including unused ones
    primal_graph: UnGraph<Qubit, Coupler, u32>,

    /// The dual graph consists of routers and edges
    dual_graph: UnGraph<Router, RouterEdge, u32>,

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
}

impl TryFrom<Config> for SearchGraph {
    type Error = anyhow::Error;
    fn try_from(value: Config) -> Result<Self, Self::Error> {
        let topology = &value.topology;
        let qubit_at_origin = topology.qubit_at_origin;
        let full_grid =
            (0..topology.grid_height).flat_map(|y| (0..topology.grid_width).map(move |x| (x, y)));
        let mut qubits = IndexMap::new();
        let mut routers = Vec::new();
        // Get all qubits and routers
        for cell in full_grid {
            if is_qubit(cell.0, cell.1, qubit_at_origin) {
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
            } else {
                routers.push(Router(cell.0, cell.1));
            }
        }

        // Create primal graph
        let mut primal_graph = UnGraph::default();
        for qubit in qubits.values() {
            primal_graph.add_node(*qubit);
        }
        for (position, qubit) in &qubits {
            for direction in [Direction::UL, Direction::UR] {
                let position2 = direction.apply(*position);
                if let Some(qubit2) = qubits.get(&position2) {
                    let q1 = qubit.qid;
                    let q2 = qubit2.qid;
                    let coupler_is_unused = topology.unused_couplers.contains(&(q1, q2))
                        || topology.unused_couplers.contains(&(q2, q1))
                        || !qubit.used
                        || !qubit2.used;
                    let coupler = Coupler {
                        q1,
                        q2,
                        used: !coupler_is_unused,
                    };
                    primal_graph.add_edge(q1.into(), q2.into(), coupler);
                }
            }
        }

        // Create dual graph
        let dual_graph = UnGraph::default();

        Ok(SearchGraph {
            primal_graph,
            dual_graph,
            config: value.algorithm,
        })
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
    use super::*;
    use petgraph::graph::NodeIndex;

    #[test]
    fn test_search_graph_basic() {
        let mut config = Config::default();

        config.topology.unused_qubits.push(1);
        let graph: SearchGraph = config.try_into().unwrap();
        assert_eq!(graph.used_qubits(), 66 - 1);
        assert_eq!(graph.used_couplers(), 110 - 2);
        let primal_graph = &graph.primal_graph;
        assert!(!primal_graph[NodeIndex::from(1)].used);
        assert!(primal_graph[NodeIndex::from(55)].used);
    }
}