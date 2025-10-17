#![deny(unsafe_code)]

mod cli;

use clap::Parser as _;

use pentas::{self, Config, Runner};

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    let config = Config {
        no_window_html: args.no_window_html,
        no_window_css: args.no_window_css,
        tree_trace_level: args.tree_trace.into(),
    };

    Runner::new(config).run()?;
    Ok(())
}
