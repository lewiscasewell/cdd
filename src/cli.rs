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
                .num_args(1) // Allows multiple occurrences, each with a single value
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
                .help("Path to tsconfig.json for resolving path aliases")
                .num_args(1),
        )
        .arg(
            Arg::new("watch")
                .short('w')
                .long("watch")
                .help("Watch mode: re-run analysis when files change")
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
        watch: *matches.get_one::<bool>("watch").unwrap_or(&false),
    }
}
