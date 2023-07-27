use crate::config::AlgorithmConfig;
use crate::cutline::Cutline;
use crate::graph::{duality_map, Point, SearchGraph};
use crate::pattern::{Order, Pattern};
use fxhash::FxHashMap as HashMap;
use fxhash::FxHashSet as HashSet;
use itertools::Itertools;
use rayon::prelude::*;
use smallvec::SmallVec;

const NEIGHBORS: &[(i32, i32)] = &[(1, 1), (1, -1), (-1, 1), (-1, -1)];

type Edge = (Point, Point);

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
    patterns
        // .into_iter()
        .into_par_iter()
        .map(|pattern| calculate_min_cost(graph, pattern, &cutlines, &ordering, &order_counts))
        .max_by(|&(_, c1), &(_, c2)| c1.partial_cmp(&c2).unwrap())
        .map(|(i, _)| cutlines[i].clone())
        .unwrap()
}

fn calculate_min_cost<P>(
    graph: &SearchGraph,
    pattern: P,
    cutlines: &[Cutline],
    ordering: &[Order],
    order_counts: &HashMap<Order, usize>,
) -> (usize, f64)
where
    P: Pattern,
{
    let order_map = pattern.order_map(graph);
    let point_order_map: HashMap<_, _> = graph
        .primal
        .nodes()
        .map(|n| (n, all_order_for_point(&order_map, n)))
        .collect();
    cutlines
        .iter()
        .map(|cutline| {
            cost_for_cutline(
                &order_map,
                &point_order_map,
                cutline,
                ordering,
                order_counts,
            )
        })
        .enumerate()
        .min_by(|&(_, c1), &(_, c2)| c1.partial_cmp(&c2).unwrap())
        .unwrap()
}

fn cost_for_cutline(
    order_map: &HashMap<Edge, Option<Order>>,
    point_order_map: &HashMap<Point, SmallVec<[(Order, Edge); 4]>>,
    cutline: &Cutline,
    ordering: &[Order],
    order_counts: &HashMap<Order, usize>,
) -> f64 {
    let path = &cutline.path;
    let cut_edges: SmallVec<[Edge; 20]> = path
        .iter()
        .tuple_windows()
        .map(|(&n1, &n2)| {
            let (n1, n2) = duality_map(n1, n2);
            (n1.min(n2), n1.max(n2))
        })
        .collect();

    // map order to cutline edges
    // TODO: attempt to remove the hashmap
    let order_to_edges: HashMap<Order, Vec<(Point, Point)>> = Order::all_possibles()
        .map(|order| {
            let edges = get_edges_for_order(order_map, &cut_edges, order);
            (order, edges)
        })
        .collect();
    // total two qubits gates on the cut
    let length: usize = order_to_edges
        .iter()
        .map(|(order, edges)| {
            let count = order_counts[order];
            count * edges.len()
        })
        .sum();
    // each gates can only be used in one optimization
    // TODO: attempt vec instead of HashSet
    let mut used_gates: HashSet<(usize, (Point, Point))> = HashSet::default();
    // start and end elision
    let start_end_elision: usize = [
        (0, ordering.first().unwrap()),
        (ordering.len() - 1, ordering.last().unwrap()),
    ]
    .into_iter()
    .map(|(depth, order)| {
        let start_end_gates = &order_to_edges[order];
        used_gates.extend(start_end_gates.clone().into_iter().map(|o| (depth, o)));
        start_end_gates.len()
    })
    .sum();
    // DCD fusion, only consider 3 gates fusion though
    // higher order fusion may exsit
    let mut n_dcd: usize = 0;
    let potential_dcd = ordering
        .windows(3)
        .enumerate()
        .filter(|&(_, window)| window[0] == window[2]);
    for (i, dcd) in potential_dcd {
        let d_gates = &order_to_edges[&dcd[0]];
        for &d_gate in d_gates {
            if used_gates.contains(&(i, d_gate)) || used_gates.contains(&(i + 2, d_gate)) {
                continue;
            }
            let (n1, n2) = d_gate;
            let mut n1_orders = &point_order_map[&n1];
            let mut n2_orders = &point_order_map[&n2];
            if !n1_orders.iter().any(|(order, _)| *order == dcd[1]) {
                std::mem::swap(&mut n1_orders, &mut n2_orders);
            }
            if let Some((_, edge)) = n1_orders.into_iter().find(|(order, _)| *order == dcd[1]) {
                if n2_orders.iter().any(|(order, _)| *order == dcd[1]) {
                    continue;
                }
                if cut_edges.contains(edge) {
                    n_dcd += 1;
                    used_gates.insert((i + 1, *edge));
                }
                used_gates.insert((i, d_gate));
                used_gates.insert((i + 2, d_gate));
                n_dcd += 1;
            } else {
                continue;
            }
        }
    }
    // wedge fusion
    let mut n_wedge: usize = 0;
    let potential_wedge = ordering.windows(2).enumerate();
    for (i, wedge) in potential_wedge {
        let (order1, order2) = (wedge[0], wedge[1]);
        let order1_edges = &order_to_edges[&order1];
        let order2_edges = &order_to_edges[&order2];
        for &edge1 in order1_edges {
            if used_gates.contains(&(i, edge1)) {
                continue;
            }
            let wedges = order2_edges
                .iter()
                .filter(|&e| e.0 == edge1.0 || e.0 == edge1.1 || e.1 == edge1.0 || e.1 == edge1.1);
            for &edge2 in wedges {
                if used_gates.contains(&(i + 1, edge2)) {
                    continue;
                }
                used_gates.insert((i, edge1));
                used_gates.insert((i + 1, edge2));
                n_wedge += 1;
                break;
            }
        }
    }

    let unbalance = cutline.unbalance as f64;
    (2f64.powf(unbalance / 2f64) + 2f64.powf(-unbalance / 2f64))
        * 4f64.powf((length - n_dcd - n_wedge) as f64 - start_end_elision as f64 / 2f64)
}

fn get_edges_for_order(
    order_map: &HashMap<(Point, Point), Option<Order>>,
    edges: &[(Point, Point)],
    order: Order,
) -> Vec<(Point, Point)> {
    edges
        .iter()
        .filter(|&edge| {
            if let Some(edge_order) = order_map[edge] {
                edge_order == order
            } else {
                false
            }
        })
        .copied()
        .collect_vec()
}

fn all_order_for_point(
    order_map: &HashMap<(Point, Point), Option<Order>>,
    point: Point,
) -> SmallVec<[(Order, (Point, Point)); 4]> {
    NEIGHBORS
        .iter()
        .filter_map(|&offset| {
            let neighbor = (point.0 + offset.0, point.1 + offset.1);
            let edge = (neighbor.min(point), neighbor.max(point));
            order_map
                .get(&edge)
                .copied()
                .flatten()
                .map(|order| (order, edge))
        })
        .collect()
}
