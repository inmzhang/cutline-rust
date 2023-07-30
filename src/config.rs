use crate::pattern::Order;
use anyhow::Result;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Builder, Clone)]
pub struct TopologyConfig {
    #[builder(default = "12")]
    pub width: u32,
    #[builder(default = "11")]
    pub height: u32,
    #[builder(default = "Vec::new()")]
    pub unused_qubits: Vec<u32>,
    #[builder(default = "Vec::new()")]
    pub unused_couplers: Vec<(u32, u32)>,
    #[builder(default = "false")]
    pub qubit_at_origin: bool,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        TopologyConfigBuilder::default().build().unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Builder, Clone)]
pub struct AlgorithmConfig {
    #[builder(default = "2")]
    pub min_depth: usize,
    #[builder(default = "10")]
    pub max_depth: usize,
    #[builder(default = "11")]
    pub max_unbalance: usize,
    #[builder(default = "vec![
            Order::A, Order::B, Order::C, Order::D, Order::C, Order::D, Order::A, Order::B, 
            Order::A, Order::B, Order::C, Order::D, Order::C, Order::D, Order::A, Order::B, 
            Order::A, Order::B, Order::C, Order::D]")]
    pub ordering: Vec<Order>,
    #[builder(default = "None")]
    pub patterns: Option<Vec<String>>,
    #[builder(default = "usize::MAX")]
    pub max_patterns: usize,
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        AlgorithmConfigBuilder::default().build().unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Config {
    pub topology: TopologyConfig,
    pub algorithm: AlgorithmConfig,
}

impl Config {
    pub fn new(topology: TopologyConfig, algorithm: AlgorithmConfig) -> Self {
        Config {
            topology,
            algorithm,
        }
    }

    pub fn save_to_json(&self, path: &Path) -> Result<()> {
        // create file if not exist
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    pub fn try_from_file(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let config: Self = serde_json::from_reader(file)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_read_write() {
        let config = Config::default();

        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        config.save_to_json(path.as_path()).unwrap();

        let config2: Config = serde_json::from_reader(File::open(path.as_path()).unwrap()).unwrap();
        assert_eq!(config, config2);
    }
}
