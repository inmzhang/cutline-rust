use crate::graphmap::{Point, SearchGraph};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Order {
    A,
    B,
    C,
    D,
}

impl Order {
    pub fn all_possibles() -> impl Iterator<Item = Order> {
        [Order::A, Order::B, Order::C, Order::D].into_iter()
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Order::A => "A",
            Order::B => "B",
            Order::C => "C",
            Order::D => "D",
        }
    }
}

pub trait Query {
    fn look_up(&self, n1: Point, n2: Point, graph: &SearchGraph) -> Option<Order>;
}
