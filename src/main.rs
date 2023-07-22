mod config;
mod graph;
mod pattern;
mod search_pattern;
use lazy_static::lazy_static;

lazy_static! {
    static ref GRAPH: graph::SearchGraph = {
        let topo = config::TopologyConfigBuilder::default()
            .qubit_at_origin(true)
            .grid_width(5)
            .grid_height(4)
            .unused_qubits(vec![])
            .build()
            .unwrap();
        graph::SearchGraph::from_config(topo).unwrap()
    };
}

fn main() {
    let vec_patterns = search_pattern::search_vec_patterns(&GRAPH);
    let bit_patterns = search_pattern::search_bit_patterns(&GRAPH);
    println!(
        "Found {} patterns in total, {} patterns for bit pattern",
        vec_patterns.len(),
        bit_patterns.count()
    )
}
