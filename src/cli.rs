use clap::{Arg, ArgAction, Command};

/// Command-line interface configuration.
pub struct Cli {
    pub dir: String,
    pub exclude: Vec<String>,
    pub debug: bool,
    /// Expected number of cycles. None means not specified on CLI.
    pub number_of_cycles: Option<usize>,
    pub silent: bool,
    pub ignore_type_imports: bool,
    pub tsconfig_path: Option<String>,
    pub watch: bool,
    /// Disable auto-detection of monorepo workspaces.
    pub no_workspace: bool,
    /// Disable auto-detection of tsconfig.json.
    pub no_tsconfig: bool,
    /// Output results as JSON.
    pub json: bool,
    /// Expected hash of all cycles for CI validation.
    pub expected_hash: Option<String>,
    /// Path to allowlist file for accepted cycles.
    pub allowlist: Option<String>,
    /// Update the expected_hash in the config file with the current hash.
    pub update_hash: bool,
    /// Initialize a .cddrc.json config file with current cycles as baseline.
    pub init: bool,
}

/// Parses command-line arguments and returns a [`Cli`] configuration.
pub fn parse_args() -> Cli {
    let matches = Command::new("Circular Dependency Detector")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Lewis Casewell")
        .about("Detects circular dependencies in your project")
        .arg(
            Arg::new("dir")
                .help("The root directory to analyze")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("exclude")
                .short('e')
                .long("exclude")
                .help("Directories to exclude")
                .num_args(1)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debug logging")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("number_of_cycles")
                .short('n')
                .long("numberOfCycles")
                .help("Specify the expected number of cycles [default: 0]")
                .num_args(1)
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("silent")
                .short('s')
                .long("silent")
                .help("Enable silent output")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("ignore_type_imports")
                .short('t')
                .long("ignore-type-imports")
                .help("Ignore type-only imports (import type { Foo }). These are erased at compile time and don't cause runtime cycles.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("tsconfig")
                .long("tsconfig")
                .help("Path to tsconfig.json (auto-detected by default)")
                .num_args(1),
        )
        .arg(
            Arg::new("no_tsconfig")
                .long("no-tsconfig")
                .help("Disable auto-detection of tsconfig.json")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("watch")
                .short('w')
                .long("watch")
                .help("Watch mode: re-run analysis when files change")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no_workspace")
                .long("no-workspace")
                .help("Disable auto-detection of monorepo workspaces")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Output results as JSON (includes line numbers and import statements)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("expected_hash")
                .long("expected-hash")
                .help("Expected hash of all cycles for CI validation. Fails if actual hash differs.")
                .num_args(1),
        )
        .arg(
            Arg::new("allowlist")
                .long("allowlist")
                .help("Path to allowlist file for accepted cycles")
                .num_args(1),
        )
        .arg(
            Arg::new("update_hash")
                .long("update-hash")
                .help("Update the expected_hash in the config file (.cddrc.json) with the current hash")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("init")
                .long("init")
                .help("Initialize .cddrc.json with current cycles as allowed baseline")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    Cli {
        dir: matches
            .get_one::<String>("dir")
            .expect("dir is a required argument")
            .to_string(),
        exclude: matches
            .get_many::<String>("exclude")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default(),
        debug: *matches.get_one::<bool>("debug").unwrap_or(&false),
        number_of_cycles: matches.get_one::<usize>("number_of_cycles").copied(),
        silent: *matches.get_one::<bool>("silent").unwrap_or(&false),
        ignore_type_imports: *matches
            .get_one::<bool>("ignore_type_imports")
            .unwrap_or(&false),
        tsconfig_path: matches.get_one::<String>("tsconfig").cloned(),
        no_tsconfig: *matches.get_one::<bool>("no_tsconfig").unwrap_or(&false),
        watch: *matches.get_one::<bool>("watch").unwrap_or(&false),
        no_workspace: *matches.get_one::<bool>("no_workspace").unwrap_or(&false),
        json: *matches.get_one::<bool>("json").unwrap_or(&false),
        expected_hash: matches.get_one::<String>("expected_hash").cloned(),
        allowlist: matches.get_one::<String>("allowlist").cloned(),
        update_hash: *matches.get_one::<bool>("update_hash").unwrap_or(&false),
        init: *matches.get_one::<bool>("init").unwrap_or(&false),
    }
}
