mod css;
mod html;
mod layout;
mod style;

use anyhow::Result;
use gtk4::pango;

use crate::app::VerbosityLevel;
use crate::ui::{DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH};
use crate::utils::PrintableTree as _;
use css::get_ua_style_sheet;
use css::parser::CssParser;
use css::token::CssTokenizer;
use html::dom::DocumentTree;
use html::parser::HtmlParser;
use html::token::HtmlTokenizer;

#[derive(Debug, Clone, PartialEq)]
pub enum RenderObject {
    Text {
        text: String,
        x: f64,
        y: f64,
        font_family: Vec<String>,
        font_size: f64,
        font_weight: String,
        /// 0.0 <= (r, g, b) <= 1.0
        color: (f64, f64, f64),
        /// 0.0 <= (r, g, b) <= 1.0
        decoration_color: (f64, f64, f64),
        decoration_line: Vec<String>,
        decoration_style: String,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        /// 0.0 <= (r, g, b) <= 1.0
        color: (f64, f64, f64),
        /// (top-left, top-right, bottom-right, bottom-left)
        border_radius: (f64, f64, f64, f64),
    },
}

#[derive(Debug, Clone, Default)]
pub struct RenderObjects {
    pub list: Vec<RenderObject>,
    pub max_width: f32,
    pub max_height: f32,
}

/// Returns a list of render objects and the title of the HTML document.
pub fn get_render_objects(
    html: &str,
    host_name: &str,
    viewport_width: i32,
    viewport_height: i32,
    draw_ctx: &pango::Context,
    verbosity: VerbosityLevel,
) -> Result<(RenderObjects, String)> {
    let parsed_object = HtmlParser::new(HtmlTokenizer::new(html)).parse()?;
    let style_sheets = std::iter::once(get_ua_style_sheet()?)
        .chain(parsed_object.style_sheets)
        .collect::<Vec<_>>();
    let title = parsed_object.title.unwrap_or_else(|| host_name.to_string());

    match verbosity {
        VerbosityLevel::Quiet => Ok((
            DocumentTree::build(parsed_object.dom_root)?
                .to_render_tree(style_sheets)?
                .to_box_tree(draw_ctx)?
                .clean_up()?
                .layout(viewport_width, viewport_height)?
                .to_render_objects(viewport_width, viewport_height),
            title,
        )),
        VerbosityLevel::Normal | VerbosityLevel::Verbose => {
            println!("Title: {}\n", title);
            Ok((
                DocumentTree::build(parsed_object.dom_root)?
                    .print_in_chain(verbosity)
                    .to_render_tree(style_sheets)?
                    .print_in_chain(verbosity)
                    .to_box_tree(draw_ctx)?
                    .print_in_chain(verbosity)
                    .clean_up()?
                    .print_in_chain(verbosity)
                    .layout(viewport_width, viewport_height)?
                    .print_in_chain(verbosity)
                    .to_render_objects(viewport_width, viewport_height),
                title,
            ))
        }
    }
}

/// Prints an HTML document as a box tree.
pub fn print_box_tree(
    html: &str,
    file_path: &str,
    draw_ctx: &pango::Context,
    verbosity: VerbosityLevel,
) -> Result<()> {
    let parsed_object = HtmlParser::new(HtmlTokenizer::new(html)).parse()?;
    let style_sheets = std::iter::once(get_ua_style_sheet()?)
        .chain(parsed_object.style_sheets)
        .collect::<Vec<_>>();

    match verbosity {
        VerbosityLevel::Quiet => {
            DocumentTree::build(parsed_object.dom_root)?
                .to_render_tree(style_sheets)?
                .to_box_tree(draw_ctx)?
                .clean_up()?
                .layout(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)?
                .print(verbosity);
        }
        VerbosityLevel::Normal | VerbosityLevel::Verbose => {
            println!(
                "Title: {}\n",
                parsed_object
                    .title
                    .unwrap_or_else(|| "file://".to_string() + file_path)
            );
            DocumentTree::build(parsed_object.dom_root)?
                .print_in_chain(verbosity)
                .to_render_tree(style_sheets)?
                .print_in_chain(verbosity)
                .to_box_tree(draw_ctx)?
                .print_in_chain(verbosity)
                .clean_up()?
                .print_in_chain(verbosity)
                .layout(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)?
                .print(verbosity);
        }
    }

    Ok(())
}

/// Prints a CSS document as a style sheet.
pub fn print_style_sheet(css: &str) -> Result<()> {
    CssParser::new(&CssTokenizer::new(css).tokenize()?)
        .parse()?
        .print();
    Ok(())
}
