mod css;
mod html;
mod layout;
mod style;
mod utils;

use anyhow::Result;

use crate::app::DEFAULT_WINDOW_WIDTH;
use css::get_ua_style_sheet;
use css::parser::CssParser;
use css::token::CssTokenizer;
use html::dom::DocumentTree;
use html::parser::HtmlParser;
use html::token::HtmlTokenizer;

#[derive(Debug, Clone)]
pub enum RenderObject {
    Text {
        text: String,
        x: f64,
        y: f64,
        size: f64,
        color: (f64, f64, f64),
        decoration_color: (f64, f64, f64),
        decoration_line: Vec<String>,
        decoration_style: String,
    },
    Rectangle {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: (f64, f64, f64),
    },
}

#[derive(Debug)]
pub struct Renderer;

impl Renderer {
    // Returns the render objects.
    pub fn run(html: &str, is_tracing_enabled: bool) -> Result<Vec<RenderObject>> {
        let (doc_root, style_sheets) = HtmlParser::new(HtmlTokenizer::new(html)).parse()?;
        let style_sheets = std::iter::once(get_ua_style_sheet()?)
            .chain(style_sheets)
            .collect::<Vec<_>>();

        if is_tracing_enabled {
            Ok(DocumentTree::build(doc_root)?
                .print_in_chain()
                .to_render_tree(style_sheets)?
                .print_in_chain()
                .to_box_tree()?
                .print_in_chain()
                .clean_up()?
                .print_in_chain()
                .layout(DEFAULT_WINDOW_WIDTH as f32)?
                .print_in_chain()
                .to_render_objects())
        } else {
            Ok(DocumentTree::build(doc_root)?
                .to_render_tree(style_sheets)?
                .to_box_tree()?
                .clean_up()?
                .layout(DEFAULT_WINDOW_WIDTH as f32)?
                .to_render_objects())
        }
    }

    /// Displays the HTML as a box tree.
    pub fn display_html(html: &str, is_tracing_enabled: bool) -> Result<()> {
        let (doc_root, style_sheets) = HtmlParser::new(HtmlTokenizer::new(html)).parse()?;

        let style_sheets = std::iter::once(get_ua_style_sheet()?)
            .chain(style_sheets)
            .collect::<Vec<_>>();

        if is_tracing_enabled {
            DocumentTree::build(doc_root)?
                .print_in_chain()
                .to_render_tree(style_sheets)?
                .print_in_chain()
                .to_box_tree()?
                .print_in_chain()
                .clean_up()?
                .print_in_chain()
                .layout(DEFAULT_WINDOW_WIDTH as f32)?
                .print();
        } else {
            DocumentTree::build(doc_root)?
                .to_render_tree(style_sheets)?
                .to_box_tree()?
                .clean_up()?
                .layout(DEFAULT_WINDOW_WIDTH as f32)?
                .print();
        }

        Ok(())
    }

    /// Displays the CSS as a style sheet.
    pub fn display_css(css: &str) -> Result<()> {
        CssParser::new(&CssTokenizer::new(css).tokenize()?)
            .parse()?
            .print();
        Ok(())
    }
}
