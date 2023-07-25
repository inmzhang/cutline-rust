mod config;
mod cost;
mod cutline;
mod graph;
mod pattern;
mod search_pattern;

use config::{AlgorithmConfigBuilder, TopologyConfigBuilder};
use graph::SearchGraph;
use cutline::search_cutlines;
use itertools::Itertools;
use search_pattern::search_bit_patterns;
use cost::max_min_cost;

fn main() {
    let topo = TopologyConfigBuilder::default()
        .grid_width(7)
        .grid_height(6)
        .unused_qubits(vec![])
        .build()
        .unwrap();
    let graph = SearchGraph::from_config(topo).unwrap();

    let algo = AlgorithmConfigBuilder::default()
        .min_search_depth(0)
        .max_search_depth(6)
        .max_unbalance(6)
        .build()
        .unwrap();

    let patterns = search_bit_patterns(&graph).collect_vec();
    let cutlines = search_cutlines(&graph, &algo);
    println!("Found {} cutlines", cutlines.len());
    // dbg!(cutlines.clone());
    let optimal_cutline = max_min_cost(&graph, patterns, cutlines, &algo);
    println!("Optimal cutline: {:?}", optimal_cutline);
}
