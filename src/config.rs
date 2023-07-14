use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TopologyConfig {
    pub grid_width: u32,
    pub grid_height: u32,
    pub unused_qubits: Vec<u32>,
    pub unused_couplers: Vec<(u32, u32)>,
    pub qubit_at_origin: bool,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        TopologyConfig {
            grid_width: 12,
            grid_height: 11,
            unused_qubits: vec![],
            unused_couplers: vec![],
            qubit_at_origin: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    pub circuit_depth: u32,
    pub max_search_depth: u32,
    pub max_unbalance: u32,
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        AlgorithmConfig {
            circuit_depth: 20,
            max_search_depth: 10,
            max_unbalance: 11,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub topology: TopologyConfig,
    pub algorithm: AlgorithmConfig,
}
