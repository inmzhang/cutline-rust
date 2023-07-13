mod config;

use std::fs::File;
use std::path::Path;
use crate::config::Config;

fn main() {
    let config = Config::default();
    // create a new path
    
    let config_filepath = Path::new("config.json");
    let config_file = File::create(config_filepath).expect("Unable to create file");
    serde_json::to_writer_pretty(config_file, &config).expect("Unable to write config");
    let config_file = File::open(config_filepath).expect("Unable to create file");
    let reload_config: Config = serde_json::from_reader(config_file).expect("Unable to read config");
    println!("{:#?}", reload_config);
}
