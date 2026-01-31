mod cli;
mod config;
mod filesystem;
mod graph;
mod output;
mod parser;
mod tsconfig;
mod utils;
mod watch;
mod workspace;

use ::colored::*;
use config::{find_config, update_config_hash, MergedConfig};
use env_logger::Builder;
use graph::CycleInfo;
use log::info;
use output::{
    compute_cycles_hash, filter_allowed_cycles, generate_json_output, print_cycles_detailed,
    print_json_error, print_json_output, OutputFormat,
};
use parser::ParserOptions;
use std::path::Path;
use std::time::Instant;
use tsconfig::{load_tsconfig, PathAliases};
use workspace::Workspace;

/// Result of cycle analysis
struct AnalysisResult {
    /// Cycles after filtering allowed ones
    filtered_cycles: Vec<CycleInfo>,
    /// Total number of files analyzed
    total_files: usize,
    /// Hash of all cycles (computed before filtering)
    cycles_hash: String,
}

fn main() {
    let cli = cli::parse_args();

    // For JSON output, we don't initialize normal logging
    let output_format = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    };

    // Only initialize logger for non-JSON output
    if output_format != OutputFormat::Json {
        initialize_logger(cli.debug, cli.silent);
    }

    // Validate that the input directory exists
    let dir_path = Path::new(&cli.dir);
    if !dir_path.exists() {
        if output_format == OutputFormat::Json {
            print_json_error(&format!("Directory '{}' does not exist", cli.dir));
        } else {
            eprintln!(
                "{} Directory '{}' does not exist",
                "Error:".red().bold(),
                cli.dir
            );
        }
        std::process::exit(1);
    }
    if !dir_path.is_dir() {
        if output_format == OutputFormat::Json {
            print_json_error(&format!("'{}' is not a directory", cli.dir));
        } else {
            eprintln!("{} '{}' is not a directory", "Error:".red().bold(), cli.dir);
        }
        std::process::exit(1);
    }

    // Load config file if present
    let canonical_dir = dir_path
        .canonicalize()
        .unwrap_or_else(|_| dir_path.to_path_buf());

    let file_config = find_config(&canonical_dir).map(|(_, config)| config);

    // Merge CLI args with config file (CLI takes precedence)
    let merged = MergedConfig::new(
        cli.exclude,
        cli.ignore_type_imports,
        cli.number_of_cycles,
        cli.tsconfig_path,
        cli.expected_hash,
        cli.allowlist,
        file_config,
    );

    // Auto-detect or load tsconfig
    let path_aliases = if cli.no_tsconfig {
        None
    } else if let Some(ref tsconfig_path) = merged.tsconfig_path {
        // Explicit path provided
        let path = Path::new(tsconfig_path);
        let aliases = load_tsconfig(path);
        if aliases.is_none() && output_format != OutputFormat::Json {
            log::warn!("Could not load tsconfig from '{}'", tsconfig_path);
        }
        aliases
    } else {
        // Auto-detect tsconfig.json
        auto_detect_tsconfig(&canonical_dir, output_format)
    };

    let parser_options = ParserOptions {
        ignore_type_imports: merged.ignore_type_imports,
    };

    // Auto-detect workspace (unless --no-workspace)
    let workspace = if cli.no_workspace {
        None
    } else {
        match Workspace::detect(&canonical_dir) {
            Some(ws) => {
                if output_format != OutputFormat::Json {
                    log::info!(
                        "Detected workspace with {} packages: {}",
                        ws.packages.len(),
                        ws.packages.keys().cloned().collect::<Vec<_>>().join(", ")
                    );
                }
                Some(ws)
            }
            None => None,
        }
    };

    if cli.watch {
        // Watch mode: run analysis and re-run on file changes
        let dir = cli.dir.clone();
        let excludes = merged.exclude.clone();
        let allowed_cycles = merged.allowed_cycles.clone();

        if let Err(e) = watch::watch_and_run(&canonical_dir, &excludes, || {
            log::info!("Starting analysis in directory: {}", dir);
            let start = Instant::now();
            let result = run_analysis(
                &dir,
                &excludes,
                &parser_options,
                path_aliases.as_ref(),
                workspace.as_ref(),
                &allowed_cycles,
                &canonical_dir,
            );
            log::info!("Analysis completed in {:.2?}", start.elapsed());

            // Print detailed output
            print_cycles_detailed(&result.filtered_cycles, &canonical_dir);

            // Check expected cycles count
            if merged.expected_cycles != result.filtered_cycles.len() {
                info!(
                    "Expected {} cycle(s), but found {} cycle(s).",
                    merged.expected_cycles.to_string().bright_green().bold(),
                    result.filtered_cycles.len().to_string().red().bold()
                );
            } else {
                info!(
                    "Expected {} cycle(s) and found {} cycle(s).",
                    merged.expected_cycles.to_string().bright_green().bold(),
                    result
                        .filtered_cycles
                        .len()
                        .to_string()
                        .bright_green()
                        .bold()
                );
            }

            // Check expected hash if specified
            if let Some(ref expected_hash) = merged.expected_hash {
                if expected_hash != &result.cycles_hash {
                    info!(
                        "Hash mismatch: expected {}, got {}",
                        expected_hash.bright_green().bold(),
                        result.cycles_hash.red().bold()
                    );
                }
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
        if output_format != OutputFormat::Json {
            log::info!("Starting analysis in directory: {}", cli.dir);
        }

        let start = Instant::now();
        let result = run_analysis(
            &cli.dir,
            &merged.exclude,
            &parser_options,
            path_aliases.as_ref(),
            workspace.as_ref(),
            &merged.allowed_cycles,
            &canonical_dir,
        );

        if output_format != OutputFormat::Json {
            log::info!("Analysis completed in {:.2?}", start.elapsed());
        }

        // Handle --init flag
        if cli.init {
            match config::init_config(&canonical_dir, &result.filtered_cycles) {
                Ok(config_path) => {
                    if output_format == OutputFormat::Json {
                        let json_output = generate_json_output(
                            &result.filtered_cycles,
                            &canonical_dir,
                            result.total_files,
                        );
                        print_json_output(&json_output);
                        eprintln!("Initialized {}", config_path.display());
                    } else {
                        info!(
                            "{} Initialized {} with {} allowed cycle(s)",
                            "OK".green().bold(),
                            config_path.display(),
                            result.filtered_cycles.len()
                        );
                        info!(
                            "All current cycles are now in the allowlist. New cycles will cause failures."
                        );
                    }
                    std::process::exit(0);
                }
                Err(e) => {
                    if output_format == OutputFormat::Json {
                        print_json_error(&e);
                    } else {
                        eprintln!("{} {}", "Error:".red().bold(), e);
                    }
                    std::process::exit(1);
                }
            }
        }

        // Handle --update-hash flag
        if cli.update_hash {
            match update_config_hash(&canonical_dir, &result.cycles_hash) {
                Ok(config_path) => {
                    if output_format == OutputFormat::Json {
                        // Include update info in JSON output
                        let json_output = generate_json_output(
                            &result.filtered_cycles,
                            &canonical_dir,
                            result.total_files,
                        );
                        print_json_output(&json_output);
                        eprintln!("Updated expected_hash in {}", config_path.display());
                    } else {
                        print_cycles_detailed(&result.filtered_cycles, &canonical_dir);
                        info!(
                            "{} Updated expected_hash to {} in {}",
                            "OK".green().bold(),
                            result.cycles_hash.bright_green().bold(),
                            config_path.display()
                        );
                    }
                    std::process::exit(0);
                }
                Err(e) => {
                    if output_format == OutputFormat::Json {
                        print_json_error(&e);
                    } else {
                        eprintln!("{} {}", "Error:".red().bold(), e);
                    }
                    std::process::exit(1);
                }
            }
        }

        // Determine exit code based on validation
        let mut exit_code = 0;

        if output_format == OutputFormat::Json {
            // JSON output mode
            let json_output =
                generate_json_output(&result.filtered_cycles, &canonical_dir, result.total_files);
            print_json_output(&json_output);

            // Still validate and set exit code
            if merged.expected_cycles != result.filtered_cycles.len() {
                exit_code = 1;
            }
            if let Some(ref expected_hash) = merged.expected_hash {
                if expected_hash != &result.cycles_hash {
                    exit_code = 1;
                }
            }
        } else {
            // Text output mode - use detailed output
            print_cycles_detailed(&result.filtered_cycles, &canonical_dir);

            // Show hash for reference
            if !result.filtered_cycles.is_empty() {
                info!("Cycles hash: {}", result.cycles_hash.dimmed());
            }

            // Check expected cycles count
            if merged.expected_cycles != result.filtered_cycles.len() {
                info!(
                    "{} Expected {} cycle(s), but found {} cycle(s).",
                    "X".red().bold(),
                    merged.expected_cycles.to_string().bright_green().bold(),
                    result.filtered_cycles.len().to_string().red().bold()
                );
                exit_code = 1;
            } else {
                info!(
                    "{} Expected {} cycle(s) and found {} cycle(s).",
                    "OK".green().bold(),
                    merged.expected_cycles.to_string().bright_green().bold(),
                    result
                        .filtered_cycles
                        .len()
                        .to_string()
                        .bright_green()
                        .bold()
                );
            }

            // Check expected hash if specified
            if let Some(ref expected_hash) = merged.expected_hash {
                if expected_hash != &result.cycles_hash {
                    info!(
                        "{} Hash mismatch: expected {}, got {}",
                        "X".red().bold(),
                        expected_hash.bright_green().bold(),
                        result.cycles_hash.red().bold()
                    );
                    exit_code = 1;
                } else {
                    info!(
                        "{} Hash matches: {}",
                        "OK".green().bold(),
                        expected_hash.bright_green().bold()
                    );
                }
            }
        }

        std::process::exit(exit_code);
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
    allowed_cycles: &[config::AllowedCycle],
    root: &Path,
) -> AnalysisResult {
    // Collect all relevant files
    let files = filesystem::collect_files(dir, excludes);
    let total_files = files.len();
    info!("Collected {} files.", total_files);

    // Build the dependency graph
    let graph = graph::build_dependency_graph(&files, parser_options, path_aliases, workspace);
    info!(
        "Built dependency graph with {} nodes and {} edges.",
        graph.node_count(),
        graph.edge_count()
    );

    // Detect unique cycles (pass root for stable hash computation)
    let all_cycles = graph::get_unique_cycles(&graph, root);
    let all_cycles_count = all_cycles.len();

    // Compute hash before filtering
    let cycles_hash = compute_cycles_hash(&all_cycles);

    // Filter allowed cycles
    let filtered_cycles = if allowed_cycles.is_empty() {
        all_cycles
    } else {
        let filtered = filter_allowed_cycles(all_cycles, allowed_cycles, root);
        if filtered.len() < all_cycles_count {
            info!(
                "Filtered {} allowed cycle(s), {} remaining.",
                all_cycles_count - filtered.len(),
                filtered.len()
            );
        }
        filtered
    };

    AnalysisResult {
        filtered_cycles,
        total_files,
        cycles_hash,
    }
}

/// Auto-detect tsconfig.json in the project directory.
fn auto_detect_tsconfig(dir: &Path, output_format: OutputFormat) -> Option<PathAliases> {
    let tsconfig_path = dir.join("tsconfig.json");
    if tsconfig_path.exists() {
        let aliases = load_tsconfig(&tsconfig_path);
        if aliases.is_some() && output_format != OutputFormat::Json {
            log::debug!("Auto-detected tsconfig.json");
        }
        aliases
    } else {
        None
    }
}
