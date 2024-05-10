use anyhow::{Ok, Result};
use clap::Parser as _;

use crate::cli;
use crate::css::parser::CssParser;
use crate::css::tokenizer::CssTokenizer;
use crate::html::dom::DocumentTree;
use crate::html::parser::HtmlParser;
use crate::html::tokenizer::HtmlTokenizer;

pub fn run() -> Result<()> {
    let args = cli::Args::parse();

    if args.html.is_some() {
        let html = std::fs::read_to_string(args.html.unwrap())?;
        println!(
            "{}",
            DocumentTree::build(HtmlParser::new(HtmlTokenizer::new(&html)).parse()?)?
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
