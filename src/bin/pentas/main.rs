mod cli;

use clap::Parser as _;

use pentas::{Config, Runner};

fn main() {
    let args = cli::Args::parse();
    let config = Config {
        no_window_html: args.no_window_html,
        no_window_css: args.no_window_css,
        is_tracing_enabled: args.trace,
    };

    if let Err(e) = Runner::new(config).run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
