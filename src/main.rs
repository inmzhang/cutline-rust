mod config;
mod cost;
mod cutline;
mod graph;
mod pattern;
mod search_pattern;

use anyhow::{anyhow, bail, Ok, Result};
use clap::Parser;
use config::*;
use cost::{max_min_cost, Record};
use cutline::search_cutlines;
use graph::SearchGraph;
use itertools::Itertools;
use pattern::{pattern_from_repr, pattern_repr, Order};
use petgraph::visit::{Dfs, EdgeRef};
use search_pattern::search_bit_patterns;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Set a custom config file, the settings in the config file
    /// will override all the command line arguments
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Set the grid width
    #[arg(short = 'x', long, value_name = "WIDTH")]
    width: Option<u32>,

    /// Set the grid height
    #[arg(short = 'y', long, value_name = "HEIGHT")]
    height: Option<u32>,

    /// Set the unused qubits
    #[arg(long, value_name = "UNUSED_QUBITS", num_args = 0.., value_delimiter = ',')]
    unused_qubits: Vec<u32>,

    /// Set the unused couplers
    #[arg(long, value_name = "UNUSED_COUPLERS", value_parser=parse_unused_couplers, num_args = 0.., value_delimiter = ' ')]
    unused_couplers: Vec<(u32, u32)>,

    /// Set the origin coordinate (0, 0) as qubit
    #[arg(long)]
    qubit_at_origin: bool,

    /// Set the minimum search depth of cutline
    #[arg(long, value_name = "MIN_DEPTH", default_value_t = 0)]
    min_depth: usize,

    /// Set the maximum search depth of cutline
    #[arg(long, value_name = "MAX_DEPTH")]
    max_depth: Option<usize>,

    /// Set the maximum unbalance of cutline
    #[arg(long, value_name = "MAX_UNBALANCE", default_value_t = 6)]
    max_unbalance: usize,

    /// Set the order of the pattern
    #[arg(long, value_name = "ORDER", default_value = "ABCDCDABABCDCDABABCD")]
    order: String,

    /// Set the patterns to search
    #[arg(short, long, value_name = "PATTERNS", num_args = 1..)]
    patterns: Option<Vec<String>>,

    /// Set the maximum number of patterns to be generated
    #[arg(long, value_name = "MAX_PATTERNS", default_value_t = usize::MAX)]
    max_patterns: usize,

    /// Set the file to save the log, default to current dir
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    log: Option<PathBuf>,

    /// Set the file to save the config
    #[arg(long, value_name = "CONFIG_FILE")]
    save_config: Option<PathBuf>,
}

fn parse_unused_couplers(s: &str) -> Result<(u32, u32)> {
    let s = s.trim();
    if s.starts_with('(') && s.ends_with(')') {
        let s = s[1..s.len() - 1].trim();
        let splitted = s.split(',').collect_vec();
        if splitted.len() == 2 {
            let n1 = splitted[0].parse::<u32>()?;
            let n2 = splitted[1].parse::<u32>()?;
            return Ok((n1, n2));
        }
    }
    bail!("Please specify valid unused couplers value in the form of '(q1, q2) (q3, q4)'.")
}

fn print_and_log<W: Write>(writter: &mut W, s: &str) -> Result<()> {
    println!("{}", s);
    writeln!(writter, "{}", s)?;
    Ok(())
}

fn split_part(split: &Vec<cutline::Edge>, graph: &SearchGraph) -> Vec<usize> {
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

fn record_repr(record: &Record, graph: &SearchGraph) -> String {
    format!(
        "Record {{ pattern: {}, split_part0: {:?}, cost: {:?} }}",
        pattern_repr(&record.pattern, graph.num_slash()),
        split_part(&record.cutline.split, graph),
        &record.cost,
    )
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config: Config;
    if let Some(path) = cli.config {
        config = Config::try_from_file(&path)?;
    } else {
        let width = cli
            .width
            .ok_or(anyhow! {"Width of the grid is not specified."})?;
        let height = cli
            .height
            .ok_or(anyhow! {"Height of the grid is not specified."})?;
        let ordering = cli
            .order
            .chars()
            .map(|c| Order::try_from(c).map_err(anyhow::Error::msg))
            .collect::<Result<Vec<Order>>>()?;
        let topo = TopologyConfigBuilder::default()
            .width(width)
            .height(height)
            .unused_qubits(cli.unused_qubits)
            .unused_couplers(cli.unused_couplers)
            .qubit_at_origin(cli.qubit_at_origin)
            .build()?;
        let algo = AlgorithmConfigBuilder::default()
            .min_depth(cli.min_depth)
            .max_depth(cli.max_depth.unwrap_or(width.max(height) as usize))
            .max_unbalance(cli.max_unbalance)
            .ordering(ordering)
            .patterns(cli.patterns)
            .max_patterns(cli.max_patterns)
            .build()?;
        config = Config::new(topo, algo);
    }

    if let Some(path) = cli.save_config {
        config.save_to_json(&path)?;
    }
    let log_path = if let Some(path) = cli.log {
        path
    } else {
        let mut path = std::env::current_dir()?;
        path.push(format!(
            "x{}_y{}_maxdepth{}_unbalance{}.log",
            config.topology.width,
            config.topology.height,
            config.algorithm.max_depth,
            config.algorithm.max_unbalance
        ));
        path
    };
    let log_file = File::create(log_path)?;
    let mut result = BufWriter::new(log_file);
    writeln!(&mut result, "===config information===")?;
    serde_json::to_writer_pretty(&mut result, &config)?;

    let topo = config.topology;
    let graph = SearchGraph::from_config(topo)?;
    let n_slash = graph.num_slash();
    let algo = config.algorithm;
    let patterns = if let Some(patterns) = algo.patterns.clone() {
        patterns
            .into_iter()
            .map(|ref p| pattern_from_repr(p))
            .take(algo.max_patterns)
            .collect_vec()
    } else {
        search_bit_patterns(&graph)
            .take(algo.max_patterns)
            .collect_vec()
    };

    let cutlines = search_cutlines(&graph, &algo);
    writeln!(&mut result, "\n\n===search information===")?;
    print_and_log(
        &mut result,
        &format!("- Found {} valid cutlines", cutlines.len()),
    )?;

    print_and_log(
        &mut result,
        &format!("- Search with {} patterns", patterns.len()),
    )?;

    let start_time = Instant::now();
    let optimal_cutline = max_min_cost(&graph, patterns, cutlines, &algo);
    let end_time = Instant::now();
    let elapsed_time = end_time - start_time;
    print_and_log(
        &mut result,
        &format!("- Total elapsed time: {:?}", elapsed_time),
    )?;
    print_and_log(
        &mut result,
        &format!("- Found {} optimal cutlines", optimal_cutline.len()),
    )?;

    writeln!(
        &mut result,
        "An example of optimal cutline:\n{}",
        record_repr(&optimal_cutline[0], &graph)
    )?;

    writeln!(
        &mut result,
        "\n===patterns own optimal cutlines===\n{:#?}",
        optimal_cutline
            .into_iter()
            .map(|r| pattern_repr(&r.pattern, n_slash))
            .collect_vec()
    )?;

    result.flush()?;

    Ok(())
}
