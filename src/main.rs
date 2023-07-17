mod config;
mod graph;
mod pattern_exhaustive;

use config::*;
use graph::*;
use pattern_exhaustive::*;

fn main() {
    let topo = TopologyConfigBuilder::default()
        .grid_width(7)
        .grid_height(5)
        .build()
        .unwrap();
    let config = Config {
        topology: topo,
        ..Default::default()
    };
    let graph = SearchGraph::from_config(config).unwrap();
    let primal_graph = &graph.primal_graph;
    let patterns = search_pattern(primal_graph);
    println!("{} patterns found", patterns.len());
    let p = patterns.get(11314).unwrap();
    print_pattern_in_2d(p, primal_graph);
    // .for_each(|p| {
    //     print_pattern_in_2d(p, primal_graph)
    //     // println!("{}", p.sort_in_graph_order(primal_graph))
    // });
}
