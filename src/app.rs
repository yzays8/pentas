use anyhow::{Ok, Result};
use clap::Parser as _;

use crate::cli;
use crate::html::dom::DocumentTree;
use crate::html::parser::Parser;
use crate::html::tokenizer::Tokenizer;

pub fn run() -> Result<()> {
    let args = cli::Args::parse();
    let html = std::fs::read_to_string(args.html)?;

    println!(
        "{}",
        DocumentTree::build(Parser::new(Tokenizer::new(html.to_string())).parse()?)?
    );
    Ok(())
}
