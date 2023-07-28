use crate::config::AlgorithmConfig;
use crate::cutline::{Cutline, CutlineWrapped};
use crate::graph::SearchGraph;
use crate::pattern::{BitPattern, Order, Pattern};
use fxhash::FxHashSet as HashSet;
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

impl Ord for Cost {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd<Self> for Cost {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let u1 = unbalance_modify(self.unbalance);
        let u2 = unbalance_modify(other.unbalance);
        let cut = 4f64.powf(self.cut_length() - other.cut_length());
        (u1 / u2 * cut).partial_cmp(&1f64)
    }
}

#[inline(always)]
fn unbalance_modify(unbalance: usize) -> f64 {
    let unbalance = unbalance as f64;
    2f64.powf(unbalance / 2f64) + 2f64.powf(-unbalance / 2f64)
}

impl Cost {
    #[inline(always)]
    fn cut_length(&self) -> f64 {
        (self.gates - self.dcd - self.wedge) as f64 - self.start_end as f64 / 2f64
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
    // do not use Itertool's `counts()` here for using FxHashMap
    let mut order_counts = [0usize; 4];
    ordering.iter().for_each(|item| {
        order_counts[*item as usize] += 1;
    });
    let cutlines_wrapped = cutlines
        .into_iter()
        .map(|c| c.into_wrapped(graph))
        .collect_vec();
    let costs: Vec<_> = patterns
        // .into_iter()
        .into_par_iter()
        .map(|pattern| {
            (
                pattern.clone(),
                calculate_min_cost(graph, pattern, &cutlines_wrapped, &ordering, &order_counts),
            )
        })
        .collect();
    costs
        .into_iter()
        .max_set_by(|&(_, (_, c1)), &(_, (_, c2))| c1.cmp(&c2))
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
    ordering: &[Order],
    order_counts: &[usize; 4],
) -> (usize, Cost) {
    let order_vec = pattern.order_vec(graph);
    cutlines
        .iter()
        .map(|cutline| cost_for_cutline(&order_vec, cutline, ordering, order_counts))
        .enumerate()
        .min_by(|&(_, c1), &(_, c2)| c1.cmp(&c2))
        .unwrap()
}

fn cost_for_cutline(
    order_vec: &[Option<Order>],
    cutline: &CutlineWrapped,
    ordering: &[Order],
    order_counts: &[usize; 4],
) -> Cost {
    let CutlineWrapped {
        split,
        #[allow(unused_variables)]
        unbalance,
        wedge_candidates,
        dcd_candidates,
    } = &cutline;

    // total two qubits gates on the cut
    let length: usize = split
        .iter()
        .map(|&i| {
            let order = order_vec[i].unwrap();
            order_counts[order as usize]
        })
        .sum();

    // each gates can only be used in one optimization
    let mut used_gates: HashSet<(usize, usize)> = HashSet::default();
    used_gates.reserve(length);

    // start and end swap reduction
    let mut start_end_elision: usize = 0;
    let start_order = *ordering.first().unwrap();
    let end_order = *ordering.last().unwrap();
    let depth = ordering.len() - 1;
    split.iter().for_each(|&e| {
        let order = order_vec[e].unwrap();
        if order == start_order {
            used_gates.insert((0, e));
            start_end_elision += 1;
        }
        if order == end_order {
            used_gates.insert((depth, e));
            start_end_elision += 1;
        }
    });

    // Wedge fusion
    let mut n_wedge: usize = 0;
    ordering.windows(2).enumerate().for_each(|(i, wedge)| {
        let (order1, order2) = (wedge[0], wedge[1]);
        wedge_candidates.iter().for_each(|&(e1, e2)| {
            if order_vec[e1].unwrap() == order1 && order_vec[e2].unwrap() == order2 {
                if used_gates.contains(&(i, e1)) || used_gates.contains(&(i + 1, e2)) {
                    return;
                }
                used_gates.insert((i, e1));
                used_gates.insert((i + 1, e2));
                n_wedge += 1;
            }
            if order_vec[e1].unwrap() == order2 && order_vec[e2].unwrap() == order1 {
                if used_gates.contains(&(i, e2)) || used_gates.contains(&(i + 1, e1)) {
                    return;
                }
                used_gates.insert((i, e2));
                used_gates.insert((i + 1, e1));
                n_wedge += 1;
            }
        })
    });

    // DCD fusion
    let mut n_dcd: usize = 0;
    ordering
        .windows(3)
        .enumerate()
        .filter(|&(_, window)| window[0] == window[2])
        .for_each(|(i, dcd)| {
            let (order1, order2) = (dcd[0], dcd[1]);
            dcd_candidates.iter().for_each(|&(e1, e2)| {
                if order_vec[e1].unwrap() == order1
                    && order_vec[e2].unwrap() == order2
                    && !used_gates.contains(&(i, e1))
                    && !used_gates.contains(&(i + 2, e1))
                    && !used_gates.contains(&(i + 1, e2))
                {
                    used_gates.insert((i, e1));
                    used_gates.insert((i + 2, e1));
                    used_gates.insert((i + 1, e2));
                    n_dcd += 1;
                    if split.contains(&e2) {
                        n_dcd += 1;
                    }
                }
            })
        });

    Cost {
        gates: length,
        start_end: start_end_elision,
        wedge: n_wedge,
        dcd: n_dcd,
        unbalance: cutline.unbalance,
    }
}
