mod cli;

use clap::Parser as _;

use pentas::{Config, Runner};

fn main() {
    let args = cli::Args::parse();
    let config = Config {
        html_path: args.html,
        css_path: args.css,
        is_tracing_enabled: args.trace,
        is_rendering_disabled: args.no_rendering,
    };

    if let Err(e) = Runner::new(config).run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
