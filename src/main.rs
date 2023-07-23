mod config;
mod graph;
mod pattern;
mod search_pattern;
use lazy_static::lazy_static;

lazy_static! {
    static ref GRAPH: graph::SearchGraph = {
        let topo = config::TopologyConfigBuilder::default()
            .qubit_at_origin(true)
            .grid_width(4)
            .grid_height(3)
            .unused_qubits(vec![21])
            .build()
            .unwrap();
        graph::SearchGraph::from_config(topo).unwrap()
    };

    static ref ALGORITHM: config::AlgorithmConfig = {
        config::AlgorithmConfigBuilder::default()
            .circuit_depth(20)
            .max_search_depth(11)
            .max_unbalance(6)
            .build()
            .unwrap()
    };
}

fn main() {
    dbg!(&ALGORITHM.circuit_depth);
    let vec_patterns = search_pattern::search_vec_patterns(&GRAPH);
    println!("Found {} vec patterns", vec_patterns.len());
    let bit_patterns = search_pattern::search_bit_patterns(&GRAPH);
    println!("Found {} bit patterns", bit_patterns.count());
}
