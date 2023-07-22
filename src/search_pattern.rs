use crate::graphmap::{Point, SearchGraph};
use crate::order::{Order, Query};
use fixedbitset::FixedBitSet;

/// Exhaustive search pattern
pub type VecPattern = Vec<Option<Order>>;

impl Query for VecPattern {
    fn look_up(&self, n1: Point, n2: Point, graph: &SearchGraph) -> Option<Order> {
        let index = get_edge_index(n1, n2, graph.config.grid_width);
        debug_assert!(index < graph.primal.edge_count());
        self[index]
    }
}

fn get_edge_index(n1: Point, n2: Point, width: u32) -> usize {
    let index = (n1.1 + n2.1 - 1) / 2 * width + (n1.0 + n2.0 - 1) / 2;
    index as usize
}

pub type BitPattern = FixedBitSet;

impl Query for BitPattern {
    fn look_up(&self, n1: Point, n2: Point, graph: &SearchGraph) -> Option<Order> {
        let (n1, n2) = (n1.min(n2), n1.max(n2));
        let is_ab = self[0];
        let is_slash = n1.1 < n2.1;
        let qubit_at_origin = graph.config.qubit_at_origin;
        let height = graph.config.grid_height;

        let mut index: usize;
        // slash pattern
        if is_slash {
            let offset = if qubit_at_origin { 0 } else { 1 };
            index = (offset + (n1.0 + n1.1) / 2) as usize;
        } else {
            let num_slash = get_num_slash(graph);
            let offset: u32 = if qubit_at_origin {
                if height % 2 == 0 {
                    1
                } else {
                    0
                }
            } else {}
        }
        todo!()
    }
}

#[inline]
fn get_num_slash(graph: &SearchGraph) -> usize {
    let primal = &graph.primal;
    let primal_width = graph.config.grid_width;
    primal
        .nodes()
        .filter(|&n| {
            (n.1 == 0 || n.0 == primal_width)
                && primal
                    .edge_weight(n, (n.0.wrapping_sub(1), n.1 + 1))
                    .is_some()
        })
        .count()
}
