use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    grid_width: usize,
    grid_height: usize,
    broken_qubits: Vec<usize>,
    circuit_depth: usize,
    max_search_depth: usize,
    max_unbalance: usize,
    qubit_at_origin: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            grid_width: 12,
            grid_height: 11,
            broken_qubits: vec![],
            circuit_depth: 20,
            max_search_depth: 10,
            max_unbalance: 11,
            qubit_at_origin: false,
        }
    }
}