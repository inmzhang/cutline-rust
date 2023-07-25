use crate::pattern::Order;
use anyhow::Result;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Builder, Clone)]
pub struct TopologyConfig {
    #[builder(default = "12")]
    pub grid_width: u32,
    #[builder(default = "11")]
    pub grid_height: u32,
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Builder)]
pub struct AlgorithmConfig {
    #[builder(default = "20")]
    pub circuit_depth: usize,
    #[builder(default = "2")]
    pub min_search_depth: usize,
    #[builder(default = "10")]
    pub max_search_depth: usize,
    #[builder(default = "11")]
    pub max_unbalance: usize,
    #[builder(
        default = "vec![Order::A, Order::B, Order::C, Order::D, Order::C, Order::D, Order::A, Order::B]"
    )]
    pub ordering: Vec<Order>,
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        AlgorithmConfigBuilder::default().build().unwrap()
    }
}

impl AlgorithmConfig {
    pub fn full_ordering(&self) -> Vec<Order> {
        self.ordering
            .iter()
            .cycle()
            .take(self.circuit_depth)
            .cloned()
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Config {
    pub topology: TopologyConfig,
    pub algorithm: AlgorithmConfig,
}

impl Config {
    #[allow(unused)]
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
    use itertools::Itertools;
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

    #[test]
    fn test_full_ordering() {
        let config = AlgorithmConfig::default();
        assert_eq!(
            config.full_ordering(),
            "ABCDCDABABCDCDABABCD"
                .chars()
                .map(|c| Order::from(c.to_string()))
                .collect_vec()
        )
    }
}
