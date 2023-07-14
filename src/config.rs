use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs::File;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Config {
    pub topology: TopologyConfig,
    pub algorithm: AlgorithmConfig,
}

impl Config {
    pub fn save_to_json(&self, path: &Path) -> Result<()> {
        // create file if not exist
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_read_write() -> Result<()> {
        let config = Config::default();

        let dir = tempdir()?;
        let path = dir.path().join("config.json");
        config.save_to_json(path.as_path())?;

        let config2: Config = serde_json::from_reader(File::open(path.as_path())?)?;
        assert_eq!(config, config2);
        Ok(())
    }
}
