use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs::File;
use anyhow::Result;
use derive_builder::Builder;

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
    pub circuit_depth: u32,
    #[builder(default = "10")]
    pub max_search_depth: u32,
    #[builder(default = "11")]
    pub max_unbalance: u32,
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
