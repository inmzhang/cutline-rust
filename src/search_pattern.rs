use crate::{
    graphmap::SearchGraph,
    pattern::{slash_index, BitPattern},
};
use std::collections::HashSet;

pub fn all_bit_patterns(graph: &SearchGraph) -> impl Iterator<Item = BitPattern> {
    let n_slash = graph.num_slash();
    let n_back_slash = graph.num_back_slash();
    let n_bits = 1 + n_slash + n_back_slash;
    if n_bits >= 32 {
        panic!("Number of patterns is too large! The sum of number of slash and back slash should be less than 32.");
    }
    let max_num: u32 = (1 << n_bits) - 1;
    let dead_indices = dead_slash_indices(graph);
    (0..=max_num)
        .filter(move |n| dead_indices.iter().all(|&i| n & (1 << i) == 0))
        .map(move |n| BitPattern::with_capacity_and_blocks(n_bits, vec![n]))
}

fn dead_slash_indices(graph: &SearchGraph) -> Vec<usize> {
    let mut live_slash = HashSet::new();
    let n_slash = graph.num_slash();
    let n_back_slash = graph.num_back_slash();
    graph.primal.all_edges().for_each(|(n1, n2, &weight)| {
        if !weight {
            return;
        }
        let index = slash_index(
            n1,
            n2,
            graph.config.qubit_at_origin,
            graph.config.grid_height,
            n_slash,
        );
        live_slash.insert(index);
    });
    (1..=(n_slash + n_back_slash))
        .filter(|i| !live_slash.contains(i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TopologyConfig;

    macro_rules! test_n_bit_pattern {
        ($unused:expr, $nbits:expr) => {
            let mut config = TopologyConfig::default();
            config.unused_qubits.extend($unused);
            let graph = SearchGraph::from_config(config).unwrap();
            assert_eq!(all_bit_patterns(&graph).count(), 1 << $nbits);
        };
    }

    #[test]
    fn test_bit_pattern_number() {
        test_n_bit_pattern!(Vec::<u32>::new(), 21);
        test_n_bit_pattern!([6], 20);
        test_n_bit_pattern!([54, 60, 4, 5, 11, 17], 19);
    }
}