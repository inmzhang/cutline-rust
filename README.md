# Cutline-rust

Rust implementation of the "optimal SFA cutline and pattern finding" algorithm. This is a rewrite of the javascript version which is implemented by Youwei Zhao to gain a speed boost.

## Usage

```

Search for the optimal cutline of SFA algorithm with different gate patterns.

Usage: cutline-rust [OPTIONS]

Options:
  -c, --config <FILE>
          Set a custom config file, the settings in the config file will override all the command line arguments
  -x, --width <WIDTH>
          Set the grid width
  -y, --height <HEIGHT>
          Set the grid height
      --unused-qubits [<UNUSED_QUBITS>...]
          Set the unused qubits
      --unused-couplers [<UNUSED_COUPLERS>...]
          Set the unused couplers
      --qubit-at-origin
          Set the origin coordinate (0, 0) as qubit
      --min-depth <MIN_DEPTH>
          Set the minimum search depth of cutline [default: 0]
      --max-depth <MAX_DEPTH>
          Set the maximum search depth of cutline
      --max-unbalance <MAX_UNBALANCE>
          Set the maximum unbalance of cutline [default: 6]
      --order <ORDER>
          Set the order of the pattern [default: ABCDCDABABCDCDABABCD]
  -p, --patterns <PATTERNS>...
          Set the patterns to search
      --max-patterns <MAX_PATTERNS>
          Set the maximum number of patterns to be generated [default: 18446744073709551615]
  -l, --log <OUTPUT_FILE>
          Set the file to save the log, default to current dir
      --save-config <CONFIG_FILE>
          Set the file to save the config
  -h, --help
          Print help
  -V, --version
          Print version
```