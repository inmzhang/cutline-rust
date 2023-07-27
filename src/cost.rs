use crate::config::AlgorithmConfig;
use crate::cutline::{Cutline, Edge};
use crate::graph::SearchGraph;
use crate::pattern::{Order, Pattern};
use fxhash::FxHashMap as HashMap;
use fxhash::FxHashSet as HashSet;
use itertools::Itertools;
use rayon::prelude::*;

struct CutlineWrapped {
    cutline: Cutline,
    wedge_candidates: Vec<(Edge, Edge)>,
    dcd_candidates: Vec<(Edge, Edge)>,
}

fn get_wrapped_cutline(value: Cutline, graph: &SearchGraph) -> CutlineWrapped {
    let primal = &graph.primal;
    let split = value
        .split
        .into_iter()
        .filter(|e| primal.edge_weight(e.0, e.1).unwrap().to_owned())
        .collect_vec();
    let wedge_candidates = split
        .iter()
        .combinations(2)
        .filter_map(|comb| {
            let (e1, e2) = (*comb[0], *comb[1]);
            if e1.0 == e2.0 || e1.0 == e2.1 || e1.1 == e2.0 || e1.1 == e2.1 {
                Some((e1, e2))
            } else {
                None
            }
        })
        .collect_vec();
    let dcd_candidates = split
        .iter()
        .filter_map(|&(n1, n2)| {
            let incident_node1 = (2 * n1.0 - n2.0, 2 * n1.1 - n2.1);
            let incident_node2 = (2 * n2.0 - n1.0, 2 * n2.1 - n1.1);
            match (
                primal.edge_weight(n1, incident_node1).copied(),
                primal.edge_weight(n2, incident_node2).copied(),
            ) {
                (Some(true), Some(false)) | (Some(true), None) => {
                    Some(((n1, n2), (incident_node1, n1)))
                }
                (Some(false), Some(true)) | (None, Some(true)) => {
                    Some(((n1, n2), (n2, incident_node2)))
                }
                _ => None,
            }
        })
        .collect_vec();
    CutlineWrapped {
        cutline: Cutline {
            split,
            unbalance: value.unbalance,
        },
        wedge_candidates,
        dcd_candidates,
    }
}

pub fn max_min_cost<P>(
    graph: &SearchGraph,
    patterns: Vec<P>,
    cutlines: Vec<Cutline>,
    algorithm_config: &AlgorithmConfig,
) -> Cutline
where
    P: Pattern + Send,
{
    let ordering = algorithm_config.full_ordering();
    // do not use Itertool's `counts()` here for using FxHashMap
    let mut order_counts = HashMap::default();
    ordering
        .iter()
        .copied()
        .for_each(|item| *order_counts.entry(item).or_default() += 1);
    let cutlines_wrapped = cutlines
        .into_iter()
        .map(|c| get_wrapped_cutline(c, graph))
        .collect_vec();
    patterns
        // .into_iter()
        .into_par_iter()
        .map(|pattern| {
            calculate_min_cost(graph, pattern, &cutlines_wrapped, &ordering, &order_counts)
        })
        .max_by(|&(_, c1), &(_, c2)| c1.partial_cmp(&c2).unwrap())
        .map(|(i, _)| cutlines_wrapped[i].cutline.clone())
        .unwrap()
}

fn calculate_min_cost<P>(
    graph: &SearchGraph,
    pattern: P,
    cutlines: &[CutlineWrapped],
    ordering: &[Order],
    order_counts: &HashMap<Order, usize>,
) -> (usize, f64)
where
    P: Pattern,
{
    let order_map = pattern.order_map(graph);
    let length_counts: HashMap<_, _> = order_map
        .iter()
        .map(|(&edge, order)| {
            let count = order_counts[order];
            (edge, count)
        })
        .collect();
    cutlines
        .iter()
        .map(|cutline| cost_for_cutline(&order_map, cutline, ordering, &length_counts))
        .enumerate()
        .min_by(|&(_, c1), &(_, c2)| c1.partial_cmp(&c2).unwrap())
        .unwrap()
}

fn cost_for_cutline(
    order_map: &HashMap<Edge, Order>,
    cutline: &CutlineWrapped,
    ordering: &[Order],
    length_counts: &HashMap<Edge, usize>,
) -> f64 {
    let CutlineWrapped {
        cutline,
        wedge_candidates,
        dcd_candidates,
    } = &cutline;
    let cut_edges = &cutline.split;

    // total two qubits gates on the cut
    let length: usize = cut_edges.iter().map(|e| length_counts[e]).sum();

    // each gates can only be used in one optimization
    let mut used_gates: HashSet<(usize, Edge)> = HashSet::default();
    used_gates.reserve(length);

    // start and end swap reduction
    let mut start_end_elision: usize = 0;
    let start_order = *ordering.first().unwrap();
    let end_order = *ordering.last().unwrap();
    let depth = ordering.len() - 1;
    cut_edges.iter().for_each(|e| {
        let order = order_map[e];
        if order == start_order {
            used_gates.insert((0, *e));
            start_end_elision += 1;
        }
        if order == end_order {
            used_gates.insert((depth, *e));
            start_end_elision += 1;
        }
    });

    // Wedge fusion
    let mut n_wedge: usize = 0;
    ordering.windows(2).enumerate().for_each(|(i, wedge)| {
        let (order1, order2) = (wedge[0], wedge[1]);
        wedge_candidates.iter().for_each(|(e1, e2)| {
            if order_map[e1] == order1 && order_map[e2] == order2 {
                if used_gates.contains(&(i, *e1)) || used_gates.contains(&(i + 1, *e2)) {
                    return;
                }
                used_gates.insert((i, *e1));
                used_gates.insert((i + 1, *e2));
                n_wedge += 1;
            }
            if order_map[e1] == order2 && order_map[e2] == order1 {
                if used_gates.contains(&(i, *e2)) || used_gates.contains(&(i + 1, *e1)) {
                    return;
                }
                used_gates.insert((i, *e2));
                used_gates.insert((i + 1, *e1));
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
            dcd_candidates.iter().for_each(|(e1, e2)| {
                if order_map[e1] == order1
                    && order_map[e2] == order2
                    && !used_gates.contains(&(i, *e1))
                    && !used_gates.contains(&(i + 2, *e1))
                    && !used_gates.contains(&(i + 1, *e2))
                {
                    used_gates.insert((i, *e1));
                    used_gates.insert((i + 2, *e1));
                    used_gates.insert((i + 1, *e2));
                    n_dcd += 1;
                    if cut_edges.contains(e2) {
                        n_dcd += 1;
                    }
                }
            })
        });

    let unbalance = cutline.unbalance as f64;
    (2f64.powf(unbalance / 2f64) + 2f64.powf(-unbalance / 2f64))
        * 4f64.powf((length - n_dcd - n_wedge) as f64 - start_end_elision as f64 / 2f64)
}
