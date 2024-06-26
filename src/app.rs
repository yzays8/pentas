use anyhow::{bail, Ok, Result};
use clap::Parser as _;

use crate::cli;
use crate::css::parser::CssParser;
use crate::css::tokenizer::CssTokenizer;
use crate::html::dom::DocumentTree;
use crate::html::parser::HtmlParser;
use crate::html::tokenizer::HtmlTokenizer;
use crate::render_tree::RenderTree;

pub fn run() -> Result<()> {
    let args = cli::Args::parse();

    match (args.html, args.css) {
        (Some(html), None) => {
            let html = std::fs::read_to_string(html)?;
            let doc_and_css = HtmlParser::new(HtmlTokenizer::new(&html)).parse()?;
            let doc_tree = DocumentTree::build(doc_and_css.0)?;
            let style_sheets = doc_and_css.1;

            if args.parse_only {
                println!("{}", doc_tree);
            } else {
                println!("{}", RenderTree::build(doc_tree, style_sheets)?);
            }
        }
        (None, Some(css)) => {
            let css = std::fs::read_to_string(css)?;

            println!(
                "{:#?}",
                CssParser::new(CssTokenizer::new(&css).tokenize()?).parse()?
            );
        }
        _ => bail!("Provide either HTML or CSS file."),
    }

    Ok(())
}
