mod cli;
mod filesystem;
mod graph;
mod parser;

use ::colored::*;
use env_logger::Builder;
use log::info;
use parser::ParserOptions;
use std::path::PathBuf;

fn main() {
    let cli = cli::parse_args();

    initialize_logger(cli.debug, cli.silent);
    log::info!("Starting analysis in directory: {}", cli.dir);

    let parser_options = ParserOptions {
        ignore_type_imports: cli.ignore_type_imports,
    };

    let number_of_cycles = run_analysis(&cli.dir, &cli.exclude, &parser_options);

    if cli.number_of_cycles != number_of_cycles {
        info!(
            "❌ Test Failed: Expected {} cycle(s), but found {} cycle(s).",
            cli.number_of_cycles.to_string().bright_green().bold(),
            number_of_cycles.to_string().red().bold()
        );
        std::process::exit(1);
    } else {
        info!(
            "✅ Test Passed: Expected {} cycle(s) and found {} cycle(s).",
            cli.number_of_cycles.to_string().bright_green().bold(),
            number_of_cycles.to_string().bright_green().bold()
        );
        std::process::exit(0);
    }
}

/// Initializes the logger with appropriate log level based on the debug flag.
fn initialize_logger(debug: bool, silent: bool) {
    let mut builder = Builder::new();

    if silent {
        builder.filter_level(log::LevelFilter::Off);
    } else if debug {
        builder.filter_level(log::LevelFilter::Debug);
    } else {
        builder.filter_level(log::LevelFilter::Info);
    }

    builder.init();
}

fn run_analysis(dir: &str, excludes: &[String], parser_options: &ParserOptions) -> usize {
    // Canonicalize the root directory to get its absolute path
    let root = PathBuf::from(dir)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(dir));

    // Collect all relevant files
    let files = filesystem::collect_files(dir, excludes);
    info!("Collected {} files.", files.len());

    // Build the dependency graph
    let graph = graph::build_dependency_graph(&files, parser_options);
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
    cycles.len()
}
