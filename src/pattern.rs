use crate::graph::{Point, SearchGraph};
use fixedbitset::FixedBitSet;
use itertools::Itertools;
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

impl TryFrom<char> for Order {
    type Error = String;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'A' => Ok(Order::A),
            'B' => Ok(Order::B),
            'C' => Ok(Order::C),
            'D' => Ok(Order::D),
            _ => Err(format!("Invalid order: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Context {
    qubit_at_origin: bool,
    width: u32,
    height: u32,
    n_slash: usize,
}

impl Context {
    pub fn from_graph(graph: &crate::graph::SearchGraph) -> Self {
        Context {
            qubit_at_origin: graph.config.qubit_at_origin,
            width: graph.config.width,
            height: graph.config.height,
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
        let index = get_edge_index(n1, n2, (context.width - 1) as usize);
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
        let height = context.height;

        // parity == 0 => A|C , 1 => B|D
        let mut parity: bool;
        let index = slash_index(n1, n2, qubit_at_origin, height, context.n_slash);
        if is_slash {
            parity = (n1.1 % 2 == 0) ^ qubit_at_origin;
        } else {
            parity = n2.0 % 2 == 0;
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

#[allow(unused)]
pub fn pattern_repr(pattern: &BitPattern, n_slash: usize) -> String {
    let last_flip: &str = if pattern[0] { "0" } else { "1" };
    let raw = pattern.to_string();
    let (first, remain) = raw.split_at(1);
    let (middle, last) = remain.split_at(n_slash);
    vec![first, "_", middle, "_", last_flip, "_", last].join("")
}

pub fn pattern_from_repr(repr: &str) -> BitPattern {
    let splitted = repr.split('_').collect_vec();
    let bin_str = vec![splitted[0], splitted[1], splitted[3]].join("");
    let mut pattern = BitPattern::with_capacity(bin_str.len());
    for (i, c) in bin_str.char_indices() {
        if c == '1' {
            pattern.put(i);
        }
    }
    pattern
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
                qubit_at_origin: false,
                width: $graph.config.width,
                height: $graph.config.height,
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
                    .zip($orders.chars().map(|c| Order::try_from(c).unwrap()))
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
            qubit_at_origin: false,
            width: graph.config.width,
            height: graph.config.height,
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
        let edges_per_line = (graph.config.width - 1) as usize;
        assert_eq!(get_edge_index((1, 0), (0, 1), edges_per_line), 0);
        assert_eq!(get_edge_index((10, 9), (11, 10), edges_per_line), 109);
    }

    #[test]
    fn test_str_repr_of_bit_pattern() {
        let n_slash = 10;
        let mut pattern = BitPattern::with_capacity_and_blocks(21, vec![0]);
        assert_eq!(pattern.to_string(), "000000000000000000000");
        assert_eq!(pattern_repr(&pattern, n_slash), "0_0000000000_1_0000000000");
        assert_eq!(pattern_from_repr("0_0000000000_1_0000000000"), pattern);

        pattern.put(0);
        assert_eq!(pattern.to_string(), "100000000000000000000");
        assert_eq!(pattern_repr(&pattern, n_slash), "1_0000000000_0_0000000000");
        assert_eq!(pattern_from_repr("1_0000000000_0_0000000000"), pattern);
    }
}
