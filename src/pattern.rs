use crate::graph::{Point, SearchGraph};
use fixedbitset::FixedBitSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Order {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
}

impl Order {
    pub fn all_possibles() -> impl Iterator<Item = Order> {
        [Order::A, Order::B, Order::C, Order::D].into_iter()
    }
}

impl From<String> for Order {
    fn from(s: String) -> Self {
        match s.as_str() {
            "A" => Order::A,
            "B" => Order::B,
            "C" => Order::C,
            "D" => Order::D,
            _ => panic!("Invalid order: {}", s),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Context {
    edges_per_line: usize,
    qubit_at_origin: bool,
    width: u32,
    height: u32,
    n_slash: usize,
}

impl Context {
    pub fn from_graph(graph: &crate::graph::SearchGraph) -> Self {
        Context {
            edges_per_line: graph.edges_per_line(),
            qubit_at_origin: graph.config.qubit_at_origin,
            width: graph.config.grid_width,
            height: graph.config.grid_height,
            n_slash: graph.num_slash(),
        }
    }
}

pub trait Pattern {
    fn look_up(&self, n1: Point, n2: Point, context: &Context) -> Option<Order>;

    fn order_vec(&self, graph: &SearchGraph) -> Vec<Option<Order>> {
        let context = Context::from_graph(graph);
        let primal = &graph.primal;
        let mut order_vec = vec![None; primal.edge_count()];
        primal.all_edges().for_each(|(n1, n2, &is_real)| {
            if is_real {
                let order = self.look_up(n1, n2, &context);
                let index = graph.edge_index(n1, n2);
                order_vec[index] = order;
            }
        });
        order_vec
    }
}

/// Exhaustive search pattern
pub type VecPattern = Vec<Option<Order>>;

impl Pattern for VecPattern {
    fn look_up(&self, n1: Point, n2: Point, context: &Context) -> Option<Order> {
        let index = get_edge_index(n1, n2, context.edges_per_line);
        self[index]
    }
}

pub fn get_edge_index(n1: Point, n2: Point, edges_per_line: usize) -> usize {
    let index = (n1.1 + n2.1 - 1) / 2 * edges_per_line as i32 + (n1.0 + n2.0 - 1) / 2;
    index as usize
}

pub type BitPattern = FixedBitSet;

impl Pattern for BitPattern {
    fn look_up(&self, n1: Point, n2: Point, context: &Context) -> Option<Order> {
        let (n1, n2) = (n1.min(n2), n1.max(n2));
        let ab_flip_cd = self[0];
        let is_slash = n1.1 > n2.1;
        let qubit_at_origin = context.qubit_at_origin;
        let width = context.width;
        let height = context.height;

        let mut parity: bool;
        let index = slash_index(n1, n2, qubit_at_origin, height, context.n_slash);
        if is_slash {
            parity = n2.1.min(width as i32 - 1 - n2.0) % 2 == 1;
        } else {
            parity = (height as i32 - 1 - n2.1).min(width as i32 - 1 - n2.0) % 2 == 1;
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

pub fn slash_index(
    n1: Point,
    n2: Point,
    qubit_at_origin: bool,
    height: u32,
    n_slash: usize,
) -> usize {
    let (n1, n2) = (n1.min(n2), n1.max(n2));
    let is_slash = n1.1 > n2.1;
    if is_slash {
        let offset = if qubit_at_origin { 0 } else { 1 };
        (offset + (n1.0 + n1.1) / 2) as usize
    } else {
        let offset = match (qubit_at_origin, height % 2) {
            (true, 0) | (false, 1) => 1,
            _ => 0,
        };
        (offset + (height as i32 - 1 - n2.1 + n2.0) / 2) as usize + n_slash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::SearchGraph;

    macro_rules! trivial_pattern_test {
        ($graph:ident, $pattern:ident, $orders:expr) => {
            let context = Context {
                edges_per_line: $graph.edges_per_line(),
                qubit_at_origin: false,
                width: $graph.config.grid_width,
                height: $graph.config.grid_height,
                n_slash: $graph.num_slash(),
            };
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
                        assert_eq!($pattern.look_up(n, n2, &context), Some(order));
                    })
                });
        };
    }

    #[test]
    fn test_bit_pattern_look_up() {
        let graph = SearchGraph::default();
        let pattern = BitPattern::with_capacity_and_blocks(21, vec![0]);
        trivial_pattern_test!(graph, pattern, "ABCD");

        let mut pattern = BitPattern::with_capacity_and_blocks(21, vec![0]);
        pattern.put(0);
        trivial_pattern_test!(graph, pattern, "CDAB");

        let mut pattern = BitPattern::with_capacity_and_blocks(21, vec![0]);
        pattern.insert_range(..);
        trivial_pattern_test!(graph, pattern, "DCBA");

        let context = Context {
            edges_per_line: graph.edges_per_line(),
            qubit_at_origin: false,
            width: graph.config.grid_width,
            height: graph.config.grid_height,
            n_slash: graph.num_slash(),
        };
        let mut pattern = BitPattern::with_capacity_and_blocks(21, vec![0]);
        pattern.put(20);
        assert_eq!(pattern.look_up((10, 1), (11, 2), &context), Some(Order::D));
        pattern.put(0);
        assert_eq!(pattern.look_up((10, 1), (11, 2), &context), Some(Order::B));
    }

    #[test]
    fn test_get_edge_index() {
        let graph = SearchGraph::default();
        let edges_per_line = graph.edges_per_line();
        assert_eq!(get_edge_index((1, 0), (0, 1), edges_per_line), 0);
        assert_eq!(get_edge_index((10, 9), (11, 10), edges_per_line), 109);
    }
}
