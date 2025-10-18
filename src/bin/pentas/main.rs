#![deny(unsafe_code)]

mod cli;

use clap::Parser as _;

use cli::Commands::Headless;
use pentas::{self, Config, Runner};

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    let (w, h) = match args.window_size.as_slice() {
        [w, h, ..] => (*w, *h),
        _ => unreachable!(),
    };
    let (url, local_html, local_css, is_headless) = match args.command {
        Some(Headless(headless)) => (headless.url, headless.local_html, headless.local_css, true),
        None => (None, None, None, false),
    };

    let config = Config {
        window_size: (w, h),
        url,
        is_headless,
        local_html,
        local_css,
        dump_level: args.dump.into(),
    };

    Runner::new(config).run()?;
    Ok(())
}
