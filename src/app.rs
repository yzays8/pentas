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
        (Some(html), Some(css)) => {
            let html = std::fs::read_to_string(html)?;
            let css = std::fs::read_to_string(css)?;
            let doc_css = HtmlParser::new(HtmlTokenizer::new(&html)).parse()?;
            let doc_tree = DocumentTree::build(doc_css.0)?;
            let style_sheets = if doc_css.1.is_empty() {
                vec![CssParser::new(CssTokenizer::new(&css).tokenize()?).parse()?]
            } else {
                doc_css.1
            };

            println!("{}", RenderTree::build(doc_tree, style_sheets)?);
        }
        (Some(html), None) => {
            let html = std::fs::read_to_string(html)?;

            println!(
                "{}",
                DocumentTree::build(HtmlParser::new(HtmlTokenizer::new(&html)).parse()?.0)?
            );
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
