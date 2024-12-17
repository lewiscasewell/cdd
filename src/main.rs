mod cli;
mod filesystem;
mod graph;
mod parser;

use env_logger::Builder;
use log::info;
use std::path::PathBuf;

fn main() {
    let cli = cli::parse_args();

    initialize_logger(cli.debug);

    info!("Starting analysis in directory: {}", cli.dir);
    let has_cycles = run_analysis(&cli.dir, &cli.exclude);

    if has_cycles {
        std::process::exit(1);
    } else {
        std::process::exit(0);
    }
}

/// Initializes the logger with appropriate log level based on the debug flag.
fn initialize_logger(debug: bool) {
    let mut builder = Builder::new();

    if debug {
        builder.filter_level(log::LevelFilter::Debug);
    } else {
        builder.filter_level(log::LevelFilter::Info);
    }

    builder.init();
}

fn run_analysis(dir: &str, excludes: &[String]) -> bool {
    // Canonicalize the root directory to get its absolute path
    let root = PathBuf::from(dir)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(dir));

    // Collect all relevant files
    let files = filesystem::collect_files(dir, excludes);
    info!("Collected {} files.", files.len());

    // Build the dependency graph
    let graph = graph::build_dependency_graph(&files);
    info!(
        "Built dependency graph with {} nodes and {} edges.",
        graph.node_count(),
        graph.edge_count()
    );

    // Detect unique cycles
    let cycles = graph::get_unique_cycles(&graph);

    // Print cycles with relative paths
    graph::print_cycles(&cycles, &root);

    // Return true if cycles are found
    !cycles.is_empty()
}
