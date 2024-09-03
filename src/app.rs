use anyhow::{bail, Ok, Result};
use clap::Parser as _;

use crate::cli;
use crate::css::cssom::StyleSheet;
use crate::css::parser::CssParser;
use crate::css::tokenizer::CssTokenizer;
use crate::html::dom::DocumentTree;
use crate::html::parser::HtmlParser;
use crate::html::tokenizer::HtmlTokenizer;

const UA_CSS_PATH: &str = "src/css/ua.css";

pub fn run() -> Result<()> {
    let args = cli::Args::parse();

    match (args.html, args.css) {
        (Some(html), None) => {
            let (doc_root, style_sheets) =
                HtmlParser::new(HtmlTokenizer::new(&std::fs::read_to_string(html)?)).parse()?;
            let doc_tree = DocumentTree::build(doc_root)?;
            if args.trace {
                println!("{}", doc_tree);
                println!("\n===============\n");
            }

            let style_sheets = std::iter::once(get_ua_style_sheet()?)
                .chain(style_sheets)
                .collect::<Vec<_>>();

            let render_tree = doc_tree.to_render_tree(style_sheets)?;
            if args.trace {
                println!("{}", render_tree);
                println!("\n===============\n");
            }

            if args.trace {
                let mut box_tree = render_tree.to_box_tree()?;
                println!("{}", box_tree);
                println!("\n===============\n");
                println!(
                    "{}",
                    box_tree.remove_whitespace()?.remove_empty_anonymous_boxes()
                );
            } else {
                println!(
                    "{}",
                    render_tree
                        .to_box_tree()?
                        .remove_whitespace()?
                        .remove_empty_anonymous_boxes()
                );
            }
        }
        (None, Some(css)) => {
            println!(
                "{:#?}",
                CssParser::new(CssTokenizer::new(&std::fs::read_to_string(css)?).tokenize()?)
                    .parse()?
            );
        }
        _ => bail!("Provide either HTML or CSS file."),
    }

    Ok(())
}

/// Returns the user agent style sheet.
fn get_ua_style_sheet() -> Result<StyleSheet> {
    let ua_css = std::fs::read_to_string(UA_CSS_PATH)?;
    Ok(CssParser::new(CssTokenizer::new(&ua_css).tokenize().unwrap()).parse()?)
}
