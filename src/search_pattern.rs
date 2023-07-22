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
    let index = (n1.1 + n2.1 - 1) / 2 * width as i32 + (n1.0 + n2.0 - 1) / 2;
    index as usize
}

pub type BitPattern = FixedBitSet;

impl Query for BitPattern {
    fn look_up(&self, n1: Point, n2: Point, graph: &SearchGraph) -> Option<Order> {
        if !graph.primal.edge_weight(n1, n2).unwrap() {
            return None;
        }
        let (n1, n2) = (n1.min(n2), n1.max(n2));
        let ab_flip_cd = self[0];
        let is_slash = n1.1 > n2.1;
        let qubit_at_origin = graph.config.qubit_at_origin;
        let width = graph.config.grid_width as i32;
        let height = graph.config.grid_height as i32;

        let index: usize;
        let mut parity: bool;
        if is_slash {
            let offset = if qubit_at_origin { 0 } else { 1 };
            index = (offset + (n1.0 + n1.1) / 2) as usize;
            parity = n2.1.min(width - 1 - n2.0) % 2 == 1;
        } else {
            let num_slash = get_num_slash(graph);
            let offset = match (qubit_at_origin, height % 2) {
                (true, 0) | (false, 1) => 1,
                _ => 0,
            };
            index = (offset + num_slash as i32 + (height - 1 - n2.1 + n2.0) / 2) as usize;
            parity = (height - 1 - n2.1).min(width - 1 - n2.0) % 2 == 1;
        }
        parity ^= self[index];
        match (ab_flip_cd ^ is_slash, parity) {
            (false, false) => Some(Order::C),
            (false, true) => Some(Order::D),
            (true, false) => Some(Order::A),
            (true, true) => Some(Order::B),
        }
    }
}

#[inline]
fn get_num_slash(graph: &SearchGraph) -> usize {
    let primal = &graph.primal;
    let primal_width = graph.config.grid_width as i32;
    primal
        .nodes()
        .filter(|&n| {
            (n.1 == 0 || n.0 == primal_width - 1)
                && primal.edge_weight(n, (n.0 - 1, n.1 + 1)).is_some()
        })
        .count()
}

#[inline]
fn get_num_back_slash(graph: &SearchGraph) -> usize {
    let primal = &graph.primal;
    let primal_width = graph.config.grid_width as i32;
    let primal_height = graph.config.grid_height as i32;
    primal
        .nodes()
        .filter(|&n| {
            (n.1 == primal_height - 1 || n.0 == primal_width - 1)
                && primal.edge_weight(n, (n.0 - 1, n.1 - 1)).is_some()
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! trivial_pattern_test {
        ($graph:ident, $pattern:ident, $orders:expr) => {
            $graph
                .primal
                .nodes()
                .filter(|n| n.1 % 2 == 1)
                .for_each(|n| {
                    [
                        (n.0 + 1, n.1 - 1),
                        (n.0 - 1, n.1 + 1),
                        (n.0 + 1, n.1 + 1),
                        (n.0 - 1, n.1 - 1),
                    ]
                    .into_iter()
                    .zip($orders.chars().map(|c| Order::from(c.to_string())))
                    .filter(|&(n2, _)| $graph.primal.contains_edge(n, n2))
                    .for_each(|(n2, order)| {
                        assert_eq!($pattern.look_up(n, n2, &($graph)), Some(order));
                    })
                });
        };
    }

    #[test]
    fn test_bit_pattern_look_up() {
        let graph = SearchGraph::default();
        assert_eq!(get_num_slash(&graph), 10);
        assert_eq!(get_num_back_slash(&graph), 10);
        let pattern = BitPattern::with_capacity_and_blocks(21, vec![0]);
        trivial_pattern_test!(graph, pattern, "ABCD");

        let mut pattern = BitPattern::with_capacity_and_blocks(21, vec![0]);
        pattern.put(0);
        trivial_pattern_test!(graph, pattern, "CDAB");

        let mut pattern = BitPattern::with_capacity_and_blocks(21, vec![0]);
        pattern.insert_range(..);
        trivial_pattern_test!(graph, pattern, "DCBA");

        let mut pattern = BitPattern::with_capacity_and_blocks(21, vec![0]);
        pattern.put(20);
        assert_eq!(pattern.look_up((10, 1), (11, 2), &graph), Some(Order::D));
        pattern.put(0);
        assert_eq!(pattern.look_up((10, 1), (11, 2), &graph), Some(Order::B));
    }
}
