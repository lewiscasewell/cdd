mod cli;
mod config;
mod filesystem;
mod graph;
mod parser;
mod tsconfig;
mod watch;
mod workspace;

use ::colored::*;
use config::{find_config, MergedConfig};
use env_logger::Builder;
use log::info;
use parser::ParserOptions;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tsconfig::{load_tsconfig, PathAliases};
use workspace::Workspace;

fn main() {
    let cli = cli::parse_args();

    initialize_logger(cli.debug, cli.silent);

    // Validate that the input directory exists
    let dir_path = Path::new(&cli.dir);
    if !dir_path.exists() {
        eprintln!(
            "{} Directory '{}' does not exist",
            "Error:".red().bold(),
            cli.dir
        );
        std::process::exit(1);
    }
    if !dir_path.is_dir() {
        eprintln!("{} '{}' is not a directory", "Error:".red().bold(), cli.dir);
        std::process::exit(1);
    }

    // Load config file if present
    let canonical_dir = dir_path
        .canonicalize()
        .unwrap_or_else(|_| dir_path.to_path_buf());
    let file_config = find_config(&canonical_dir);

    // Merge CLI args with config file (CLI takes precedence)
    let merged = MergedConfig::new(
        cli.exclude,
        cli.ignore_type_imports,
        cli.number_of_cycles,
        cli.tsconfig_path,
        file_config,
    );

    // Load tsconfig if specified
    let path_aliases = merged.tsconfig_path.as_ref().and_then(|tsconfig_path| {
        let path = Path::new(tsconfig_path);
        let aliases = load_tsconfig(path);
        if aliases.is_none() {
            log::warn!("Could not load tsconfig from '{}'", tsconfig_path);
        }
        aliases
    });

    let parser_options = ParserOptions {
        ignore_type_imports: merged.ignore_type_imports,
    };

    // Load workspace if --workspace flag is set
    let workspace = if cli.workspace {
        match Workspace::detect(&canonical_dir) {
            Some(ws) => {
                log::info!(
                    "Detected workspace with {} packages: {}",
                    ws.packages.len(),
                    ws.packages.keys().cloned().collect::<Vec<_>>().join(", ")
                );
                Some(ws)
            }
            None => {
                log::warn!("--workspace flag set but no workspace configuration found");
                None
            }
        }
    } else {
        None
    };

    if cli.watch {
        // Watch mode: run analysis and re-run on file changes
        let dir = cli.dir.clone();
        let excludes = merged.exclude.clone();

        if let Err(e) = watch::watch_and_run(&canonical_dir, &excludes, || {
            log::info!("Starting analysis in directory: {}", dir);
            let start = Instant::now();
            let number_of_cycles =
                run_analysis(&dir, &excludes, &parser_options, path_aliases.as_ref(), workspace.as_ref());
            log::info!("Analysis completed in {:.2?}", start.elapsed());

            if merged.expected_cycles != number_of_cycles {
                info!(
                    "❌ Expected {} cycle(s), but found {} cycle(s).",
                    merged.expected_cycles.to_string().bright_green().bold(),
                    number_of_cycles.to_string().red().bold()
                );
            } else {
                info!(
                    "✅ Expected {} cycle(s) and found {} cycle(s).",
                    merged.expected_cycles.to_string().bright_green().bold(),
                    number_of_cycles.to_string().bright_green().bold()
                );
            }
        }) {
            eprintln!(
                "{} Failed to start watch mode: {}",
                "Error:".red().bold(),
                e
            );
            std::process::exit(1);
        }
    } else {
        // Single run mode
        log::info!("Starting analysis in directory: {}", cli.dir);

        let start = Instant::now();
        let number_of_cycles = run_analysis(
            &cli.dir,
            &merged.exclude,
            &parser_options,
            path_aliases.as_ref(),
            workspace.as_ref(),
        );
        log::info!("Analysis completed in {:.2?}", start.elapsed());

        if merged.expected_cycles != number_of_cycles {
            info!(
                "❌ Test Failed: Expected {} cycle(s), but found {} cycle(s).",
                merged.expected_cycles.to_string().bright_green().bold(),
                number_of_cycles.to_string().red().bold()
            );
            std::process::exit(1);
        } else {
            info!(
                "✅ Test Passed: Expected {} cycle(s) and found {} cycle(s).",
                merged.expected_cycles.to_string().bright_green().bold(),
                number_of_cycles.to_string().bright_green().bold()
            );
            std::process::exit(0);
        }
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

fn run_analysis(
    dir: &str,
    excludes: &[String],
    parser_options: &ParserOptions,
    path_aliases: Option<&PathAliases>,
    workspace: Option<&Workspace>,
) -> usize {
    // Canonicalize the root directory to get its absolute path
    let root = PathBuf::from(dir)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(dir));

    // Collect all relevant files
    let files = filesystem::collect_files(dir, excludes);
    info!("Collected {} files.", files.len());

    // Build the dependency graph
    let graph = graph::build_dependency_graph(&files, parser_options, path_aliases, workspace);
    info!(
        "Built dependency graph with {} nodes and {} edges.",
        graph.node_count(),
        graph.edge_count()
    );

    // Detect unique cycles
    let cycles = graph::get_unique_cycles(&graph);

    // Print cycles with relative paths
    graph::print_cycles(&cycles, &root);

    // Return the number of cycles found
    cycles.len()
}
