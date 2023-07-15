# Cutline-rust

Rust implementation of the "optimal SFA cutline and pattern finding" algorithm. This is a rewrite of the javascript version which is implemented by Youwei Zhao to gain a speed boost.

# Plan

- [x] construct the basic graph data structure using `petgraph`
- [ ] impl the same pattern finding algorithm as the javascript version
- [ ] impl the path searching
- [ ] impl cost calculation for a single pattern
- [ ] paralle cost calculation across patterns using `rayon`
- [ ] visualize the result using svg
- [ ] (Optional)use `leptos` framework to visualize and interact with the algorithm
- [ ] (Optional)extend the pattern finding to do exhaustive pattern search
- [ ] (Optional)cut the grid into more than two parts
