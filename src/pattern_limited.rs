use crate::graph::PrimalGraph;
use petgraph::graph::EdgeIndex;
use crate::pattern_exhaustive::{Order, Pattern};
use fixedbitset::FixedBitSet;

struct LimitedPattern(FixedBitSet);

impl Pattern for LimitedPattern {
    fn look_up(
        &self,
        edge_idx: EdgeIndex,
    ) -> Order {
        
        todo!()
    }
}

/// Limited pattern search following the method of javascript version
pub fn search_pattern_limited(primal_graph: &PrimalGraph) {}
