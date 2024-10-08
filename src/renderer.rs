mod css;
mod html;
mod layout;
mod style;
mod util;

use anyhow::Result;

use css::get_ua_style_sheet;
use css::parser::parse;
use css::token::tokenize;
use html::dom::DocumentTree;
use html::parser::HtmlParser;
use html::tokenizer::HtmlTokenizer;

pub fn display_box_tree(html: String, trace: bool) -> Result<()> {
    let (doc_root, style_sheets) =
        HtmlParser::new(HtmlTokenizer::new(&std::fs::read_to_string(html)?)).parse()?;
    let doc_tree = DocumentTree::build(doc_root)?;
    if trace {
        doc_tree.print();
        println!("\n===============\n");
    }

    let style_sheets = std::iter::once(get_ua_style_sheet()?)
        .chain(style_sheets)
        .collect::<Vec<_>>();

    let render_tree = doc_tree.to_render_tree(style_sheets)?;
    if trace {
        render_tree.print();
        println!("\n===============\n");
    }

    if trace {
        let mut box_tree = render_tree.to_box_tree()?;
        box_tree.print();
        println!("\n===============\n");
        box_tree.clean_up()?.print();
    } else {
        render_tree.to_box_tree()?.clean_up()?.print();
    }

    Ok(())
}

pub fn display_style_sheet(css: String) -> Result<()> {
    parse(&tokenize(&std::fs::read_to_string(css)?)?)?.print();
    Ok(())
}
