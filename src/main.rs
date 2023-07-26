mod config;
mod cost;
mod cutline;
mod graph;
mod pattern;
mod search_pattern;

use config::{AlgorithmConfigBuilder, TopologyConfigBuilder};
use cost::max_min_cost;
use cutline::search_cutlines;
use graph::SearchGraph;
use itertools::Itertools;
use search_pattern::search_bit_patterns;
use std::time::Instant;

fn main() {
    
    let topo = TopologyConfigBuilder::default()
        .grid_width(12)
        .grid_height(11)
        .unused_qubits(vec![])
        .build()
        .unwrap();
    let graph = SearchGraph::from_config(topo).unwrap();

    let algo = AlgorithmConfigBuilder::default()
        .min_search_depth(0)
        .max_search_depth(11)
        .max_unbalance(6)
        .build()
        .unwrap();

    let patterns = search_bit_patterns(&graph).collect_vec();
    dbg!(patterns.len());
    let cutlines = search_cutlines(&graph, &algo);
    println!("Found {} valid cutlines", cutlines.len());
    // dbg!(cutlines.clone());
    let patterns = patterns[0..20000].to_vec();
    let start_time = Instant::now();
    let optimal_cutline = max_min_cost(&graph, patterns, cutlines, &algo);
    let end_time = Instant::now();
    let elapsed_time = end_time - start_time;
    println!("Elapsed time: {:?}", elapsed_time);
    println!("Optimal cutline: {:?}", optimal_cutline);
}
