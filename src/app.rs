use anyhow::{Ok, Result};
use clap::Parser as _;

use crate::cli;
use crate::css::parser::Parser as CssParser;
use crate::css::tokenizer::Tokenizer as CssTokenizer;
use crate::html::dom::DocumentTree;
use crate::html::parser::Parser;
use crate::html::tokenizer::Tokenizer;

pub fn run() -> Result<()> {
    let args = cli::Args::parse();

    if args.html.is_some() {
        let html = std::fs::read_to_string(args.html.unwrap())?;
        println!(
            "{}",
            DocumentTree::build(Parser::new(Tokenizer::new(html.to_string())).parse()?)?
        );
    }

    if args.css.is_some() {
        let css = std::fs::read_to_string(args.css.unwrap())?;
        println!(
            "{:#?}",
            CssParser::new(CssTokenizer::new(&css).tokenize()?).parse()?
        );
    }

    Ok(())
}
