mod config;
mod graphmap;
use lazy_static::lazy_static;

lazy_static! {
    static ref GRAPH: graphmap::SearchGraph = {
        let mut config = config::TopologyConfig::default();
        config.unused_qubits.extend([33, 34]);
        graphmap::SearchGraph::from_config(config).unwrap()
    };
}

fn main() {
    dbg!(GRAPH.primal.node_count());
}