// cli.rs
use clap::{Arg, ArgAction, Command};

pub struct Cli {
    pub dir: String,
    pub exclude: Vec<String>,
    pub debug: bool,
}

pub fn parse_args() -> Cli {
    let matches = Command::new("Circular Dependency Detector")
        .version("0.1.0")
        .author("Your Name <you@example.com>")
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
        .get_matches();

    Cli {
        dir: matches
            .get_one::<String>("dir")
            .expect("dir is a required argument")
            .to_string(),
        exclude: matches
            .get_many::<String>("exclude")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_else(Vec::new),
        debug: *matches.get_one::<bool>("debug").unwrap_or(&false),
    }
}
