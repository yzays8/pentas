use anyhow::{bail, Ok, Result};
use clap::Parser as _;

use crate::cli;
use crate::renderer::{display_box_tree, display_style_sheet};

pub fn run() -> Result<()> {
    let args = cli::Args::parse();

    match (args.html, args.css) {
        (Some(html), None) => {
            display_box_tree(html, args.trace)?;
        }
        (None, Some(css)) => {
            display_style_sheet(css)?;
        }
        _ => bail!("Provide either HTML or CSS file."),
    }

    Ok(())
}
