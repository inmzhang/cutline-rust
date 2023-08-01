use std::fmt::{Debug, Write};

use crate::config::AlgorithmConfig;
use crate::cutline::{Cutline, CutlineWrapped};
use crate::graph::SearchGraph;
use crate::pattern::{BitPattern, Order, Pattern};
use fixedbitset::FixedBitSet;
use indicatif::ParallelProgressIterator;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use itertools::Itertools;
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    gates: usize,
    start_end: usize,
    wedge: usize,
    dcd: usize,
    unbalance: usize,
}

impl Cost {
    #[inline]
    fn cut_length(&self) -> f64 {
        (self.gates - self.dcd - self.wedge) as f64 - self.start_end as f64 / 2f64
    }

    #[inline]
    pub fn cost(&self) -> f64 {
        let length = self.cut_length();
        4f64.powf(length + self.unbalance as f64 / 4f64)
            + 4f64.powf(length - self.unbalance as f64 / 4f64)
    }
}

struct UsedBoard {
    flags: FixedBitSet,
    n_edges: usize,
}

impl UsedBoard {
    fn new(n_edges: usize, depth: usize) -> Self {
        Self {
            flags: FixedBitSet::with_capacity(depth * n_edges),
            n_edges,
        }
    }

    #[inline]
    fn is_used(&self, depth: usize, edge: usize) -> bool {
        self.flags[self.index(depth, edge)]
    }

    #[inline]
    fn set_used(&mut self, depth: usize, edge: usize) {
        self.flags.put(self.index(depth, edge));
    }

    #[inline]
    fn index(&self, depth: usize, edge: usize) -> usize {
        depth * self.n_edges + edge
    }

    #[inline]
    fn reset(&mut self) {
        self.flags.clear();
    }
}

#[derive(Debug, Clone)]
struct OrderInfo {
    ordering: Vec<Order>,
    order_counts: [usize; 4],
    potential_wedges: Vec<(usize, Order, Order)>,
    potential_dcds: Vec<(usize, Order, Order)>,
}

impl OrderInfo {
    fn new(ordering: &[Order]) -> Self {
        let mut order_counts = [0; 4];
        for order in ordering {
            order_counts[*order as usize] += 1;
        }
        let potential_wedges = ordering
            .windows(2)
            .enumerate()
            .filter_map(|(i, window)| {
                let (order1, order2) = (window[0], window[1]);
                match (order1.min(order2), order1.max(order2)) {
                    (Order::A, Order::B) | (Order::C, Order::D) => None,
                    _ => Some((i, order1, order2)),
                }
            })
            .collect_vec();
        let potential_dcds = ordering
            .windows(3)
            .enumerate()
            .filter_map(|(i, window)| {
                let (order1, order2, order3) = (window[0], window[1], window[2]);
                if order1 != order3 {
                    return None;
                }
                match (order1.min(order2), order1.max(order2)) {
                    (Order::A, Order::B) | (Order::C, Order::D) => Some((i, order1, order2)),
                    _ => None,
                }
            })
            .collect_vec();
        Self {
            ordering: ordering.to_vec(),
            order_counts,
            potential_wedges,
            potential_dcds,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Record {
    pub pattern: BitPattern,
    pub cutline: Cutline,
    pub cost: Cost,
}

pub fn max_min_cost(
    graph: &SearchGraph,
    patterns: Vec<BitPattern>,
    cutlines: Vec<Cutline>,
    algorithm_config: &AlgorithmConfig,
) -> Vec<Record> {
    let ordering = algorithm_config.ordering.clone();
    let order_info = OrderInfo::new(&ordering);
    let cutlines_wrapped = cutlines
        .clone()
        .into_iter()
        .map(|c| c.into_wrapped(graph))
        .collect_vec();
    // progress bar
    let n_tasks = patterns.len() as u64;
    let pb = ProgressBar::new(n_tasks);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
            write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
        })
        .progress_chars("#>-"),
    );

    let costs: Vec<_> = patterns
        .into_par_iter()
        .progress_with(pb)
        .map(|pattern| {
            (
                pattern.clone(),
                calculate_min_cost(graph, pattern, &cutlines_wrapped, &order_info),
            )
        })
        .collect();
    costs
        .into_iter()
        .max_set_by(|&(_, (_, c1)), &(_, (_, c2))| c1.cost().partial_cmp(&c2.cost()).unwrap())
        .into_iter()
        .map(|(pattern, (i, cost))| Record {
            pattern,
            cutline: Cutline::from_wrapper(cutlines_wrapped[i].clone(), graph),
            cost,
        })
        .collect_vec()
}

fn calculate_min_cost(
    graph: &SearchGraph,
    pattern: BitPattern,
    cutlines: &[CutlineWrapped],
    order_info: &OrderInfo,
) -> (usize, Cost) {
    let order_vec = pattern.order_vec(graph);
    let mut used_flags = UsedBoard::new(graph.primal.edge_count(), order_info.ordering.len());
    cutlines
        .iter()
        .map(|cutline| cost_for_cutline(&order_vec, cutline, order_info, &mut used_flags))
        .enumerate()
        .min_by(|&(_, c1), &(_, c2)| c1.cost().partial_cmp(&c2.cost()).unwrap())
        .unwrap()
}

fn cost_for_cutline(
    order_vec: &[Option<Order>],
    cutline: &CutlineWrapped,
    order_info: &OrderInfo,
    use_flags: &mut UsedBoard,
) -> Cost {
    let CutlineWrapped {
        split,
        #[allow(unused_variables)]
        unbalance,
        wedge_candidates,
        dcd_candidates,
    } = &cutline;

    let OrderInfo {
        ordering,
        order_counts,
        potential_wedges,
        potential_dcds,
    } = order_info;

    // total two qubits gates on the cut
    let length: usize = split
        .iter()
        .map(|&i| {
            let order = order_vec[i].unwrap();
            order_counts[order as usize]
        })
        .sum();

    // Wedge fusion
    let mut n_wedge: usize = 0;
    for &(i, order1, order2) in potential_wedges {
        for &(e1, e2) in wedge_candidates {
            for (e1, e2) in [(e1, e2), (e2, e1)] {
                if order_vec[e1].unwrap() == order1 && order_vec[e2].unwrap() == order2 {
                    if !use_flags.is_used(i, e1) && !use_flags.is_used(i + 1, e2) {
                        use_flags.set_used(i, e1);
                        use_flags.set_used(i + 1, e2);
                        n_wedge += 1;
                    }
                    break;
                }
            }
        }
    }

    // DCD fusion
    let mut n_dcd: usize = 0;
    for &(i, order1, order2) in potential_dcds {
        for &(e1, e2) in dcd_candidates {
            if order_vec[e1].unwrap() == order1
                && order_vec[e2].unwrap() == order2
                && !use_flags.is_used(i, e1)
                && !use_flags.is_used(i + 2, e1)
                && !use_flags.is_used(i + 1, e2)
            {
                use_flags.set_used(i, e1);
                use_flags.set_used(i + 2, e1);
                use_flags.set_used(i + 1, e2);
                n_dcd += 1;
                if split.contains(&e2) {
                    n_dcd += 1;
                }
            }
        }
    }

    // start and end swap reduction
    let mut start_end_elision: usize = 0;
    let start_order = *ordering.first().unwrap();
    let end_order = *ordering.last().unwrap();
    let depth = ordering.len() - 1;
    for &e in split {
        let order = order_vec[e].unwrap();
        if order == start_order && !use_flags.is_used(0, e) {
            use_flags.set_used(0, e);
            start_end_elision += 1;
        }
        if order == end_order && !use_flags.is_used(depth, e) {
            use_flags.set_used(depth, e);
            start_end_elision += 1;
        }
    }

    use_flags.reset();

    Cost {
        gates: length,
        start_end: start_end_elision,
        wedge: n_wedge,
        dcd: n_dcd,
        unbalance: cutline.unbalance,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{AlgorithmConfigBuilder, TopologyConfigBuilder},
        cutline::{self, search_cutlines},
        pattern::pattern_from_repr,
    };
    use petgraph::visit::{Dfs, EdgeRef};
    use std::collections::HashMap;

    use super::*;

    fn split_part(split: &[cutline::Edge], graph: &SearchGraph) -> Vec<usize> {
        let node_map: HashMap<_, _> = graph
            .primal
            .nodes()
            .enumerate()
            .map(|(i, n)| (n, i))
            .collect();
        let filtered_graph = petgraph::visit::EdgeFiltered::from_fn(&graph.primal, |e| {
            let (source, target) = (e.source(), e.target());
            !split.contains(&(source.min(target), source.max(target))) && *e.weight()
        });
        let mut dfs = Dfs::new(&filtered_graph, graph.primal.nodes().nth(1).unwrap());
        let mut part = Vec::new();
        while let Some(qubit) = dfs.next(&filtered_graph) {
            part.push(node_map[&qubit]);
        }
        part
    }

    #[test]
    fn test_cost_for_cutline() {
        let topo = TopologyConfigBuilder::default()
            .height(12)
            .unused_qubits(vec![11, 5])
            .build()
            .unwrap();
        let graph = SearchGraph::from_config(topo).unwrap();
        let order = "ABCDCDABABCDCDABABCDCDAB";
        let ordering = order
            .chars()
            .map(|c| Order::try_from(c).unwrap())
            .collect_vec();
        let algo = AlgorithmConfigBuilder::default()
            .ordering(ordering)
            .max_unbalance(20)
            .max_depth(12)
            .build()
            .unwrap();
        let pattern = pattern_from_repr("1_0011100000_0_00000011000");
        let order_vec = pattern.order_vec(&graph);
        let order_info = OrderInfo::new(&algo.ordering);
        let mut use_flags = UsedBoard::new(graph.primal.edge_count(), order_info.ordering.len());
        // let cutline = Cutline {
        //     split: vec![
        //         ((9, 2), (10, 3)),
        //         ((8, 3), (9, 4)),
        //         ((7, 4), (8, 5)),
        //         ((6, 5), (7, 6)),
        //         ((6, 7), (7, 6)),
        //         ((6, 7), (7, 8)),
        //         ((5, 8), (6, 9)),
        //         ((4, 9), (5, 10)),
        //         ((3, 10), (4, 11)),
        //     ],
        //     unbalance: 20,
        // };
        let cutline = Cutline {
            split: vec![
                ((8, 1), (9, 2)),
                ((7, 2), (8, 3)),
                ((6, 3), (7, 4)),
                ((5, 4), (6, 5)),
                ((4, 5), (5, 6)),
                ((3, 6), (4, 7)),
                ((2, 7), (3, 8)),
                ((2, 9), (3, 8)),
                ((3, 10), (4, 9)),
                ((3, 10), (4, 11)),
            ],
            unbalance: 0,
        };
        // let cutline = Cutline {
        //     split: vec![
        //         ((6, 1), (7, 0)),
        //         ((6, 1), (7, 2)),
        //         ((5, 2), (6, 3)),
        //         ((4, 3), (5, 4)),
        //         ((4, 5), (5, 4)),
        //         ((4, 5), (5, 6)),
        //         ((4, 7), (5, 6)),
        //         ((4, 7), (5, 8)),
        //         ((4, 9), (5, 8)),
        //         ((5, 10), (6, 9)),
        //         ((6, 11), (7, 10)),
        //     ],
        //     unbalance: 0,
        // };
        let cutline_wrapped = cutline.clone().into_wrapped(&graph);
        let cost = cost_for_cutline(&order_vec, &cutline_wrapped, &order_info, &mut use_flags);
        dbg!(cost.cost());
        dbg!(cost);
        let cutlines = search_cutlines(&graph, &algo);
        let mut reverse_split = cutline.split.clone();
        reverse_split.reverse();
        let reverse_cutline = Cutline {
            split: reverse_split,
            unbalance: 0,
        };
        assert!(cutlines.contains(&cutline) || cutlines.contains(&reverse_cutline));
        // let cutlines_wrapped = cutlines
        //     .into_iter()
        //     .map(|c| c.into_wrapped(&graph))
        //     .collect_vec();
        // let (i, cost) = calculate_min_cost(&graph, pattern, &cutlines_wrapped, &order_info);
        // let found = Cutline::from_wrapper(cutlines_wrapped[i].clone(), &graph);
        // dbg!(cost.cost());
        // println!("{:?}", split_part(&found.split, &graph));
        // dbg!(cost);
    }
}