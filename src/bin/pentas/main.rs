mod cli;

use clap::Parser as _;

use pentas::{self, Config, Runner};

fn main() {
    let args = cli::Args::parse();
    let config = Config {
        no_window_html: args.no_window_html,
        no_window_css: args.no_window_css,
        verbosity: match args.verbose {
            cli::VerbosityLevel::Quiet => pentas::VerbosityLevel::Quiet,
            cli::VerbosityLevel::Normal => pentas::VerbosityLevel::Normal,
            cli::VerbosityLevel::Verbose => pentas::VerbosityLevel::Verbose,
        },
    };

    if let Err(e) = Runner::new(config).run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
