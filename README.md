# Cutline-rust

Rust implementation of the "optimal SFA cutline and pattern finding" algorithm. This is a rewrite of the javascript version which is implemented by Youwei Zhao to gain a speed boost.

## Improvements

Compared to the javascript version, this implementation has some improvements:

- _Exhaustive pattern search_: previous version did not take mid-position broken qubits and rotated unparallel pattern alignment into account. This version will search for all possible patterns and calculate the optimal complexity.

- _Speedup_: speed is the main concern of this rewrite.


## Steps

1. exhaustive pattern search
2. partition search
3. parallel cost function calculation for each pattern