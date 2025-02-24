mod css;
mod html;
mod layout;
mod style;

use anyhow::Result;
use gtk4::pango;

use crate::app::TreeTraceLevel;
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
        /// RGB, 0.0 to 1.0
        color: (f64, f64, f64),
        /// RGB, 0.0 to 1.0
        decoration_color: (f64, f64, f64),
        decoration_line: Vec<String>,
        decoration_style: String,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        /// RGB, 0.0 to 1.0
        color: (f64, f64, f64),
        /// (top-left, top-right, bottom-right, bottom-left)
        border_radius: (f64, f64, f64, f64),
    },
}

#[derive(Debug, Clone, Default)]
pub struct RenderObjectsInfo {
    pub objects: Vec<RenderObject>,
    pub title: String,
    pub max_width: f32,
    pub max_height: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Renderer {
    draw_ctx: pango::Context,
    tree_trace_level: TreeTraceLevel,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ctx(draw_ctx: &pango::Context) -> Self {
        Self {
            draw_ctx: draw_ctx.clone(),
            ..Default::default()
        }
    }

    pub fn set_draw_ctx(&mut self, draw_ctx: &pango::Context) {
        self.draw_ctx = draw_ctx.clone();
    }

    pub fn set_tree_trace_level(&mut self, tree_trace_level: TreeTraceLevel) {
        self.tree_trace_level = tree_trace_level;
    }

    /// Returns a list of render objects and the title of the HTML document.
    pub fn get_render_objects_info(
        &self,
        html: &str,
        host_name: &str,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<RenderObjectsInfo> {
        let parsed_object = HtmlParser::new(HtmlTokenizer::new(html)).parse()?;
        let style_sheets = std::iter::once(get_ua_style_sheet()?)
            .chain(parsed_object.style_sheets)
            .collect::<Vec<_>>();
        let title = parsed_object.title.unwrap_or_else(|| host_name.to_string());

        match self.tree_trace_level {
            TreeTraceLevel::Silent => {
                let box_tree = DocumentTree::build(parsed_object.dom_root)?
                    .to_render_tree(style_sheets)?
                    .to_box_tree(&self.draw_ctx)?
                    .cleanup()?
                    .layout(viewport_width, viewport_height)?;
                let (max_width, max_height) = box_tree.get_max_size();
                Ok(RenderObjectsInfo {
                    objects: box_tree.to_render_objects(viewport_width, viewport_height),
                    title,
                    max_width,
                    max_height,
                })
            }
            TreeTraceLevel::Normal | TreeTraceLevel::Debug => {
                println!("Title: {}\n", title);
                let box_tree = DocumentTree::build(parsed_object.dom_root)?
                    .print_in_chain(self.tree_trace_level)
                    .to_render_tree(style_sheets)?
                    .print_in_chain(self.tree_trace_level)
                    .to_box_tree(&self.draw_ctx)?
                    .print_in_chain(self.tree_trace_level)
                    .cleanup()?
                    .print_in_chain(self.tree_trace_level)
                    .layout(viewport_width, viewport_height)?
                    .print_in_chain(self.tree_trace_level);
                let (max_width, max_height) = box_tree.get_max_size();
                Ok(RenderObjectsInfo {
                    objects: box_tree.to_render_objects(viewport_width, viewport_height),
                    title,
                    max_width,
                    max_height,
                })
            }
        }
    }

    /// Prints an HTML document as a box tree.
    pub fn print_box_tree(&self, html: &str, file_path: &str) -> Result<()> {
        let parsed_object = HtmlParser::new(HtmlTokenizer::new(html)).parse()?;
        let style_sheets = std::iter::once(get_ua_style_sheet()?)
            .chain(parsed_object.style_sheets)
            .collect::<Vec<_>>();

        match self.tree_trace_level {
            TreeTraceLevel::Silent => {
                DocumentTree::build(parsed_object.dom_root)?
                    .to_render_tree(style_sheets)?
                    .to_box_tree(&self.draw_ctx)?
                    .cleanup()?
                    .layout(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)?
                    .print(self.tree_trace_level);
            }
            TreeTraceLevel::Normal | TreeTraceLevel::Debug => {
                println!(
                    "Title: {}\n",
                    parsed_object
                        .title
                        .unwrap_or_else(|| "file://".to_string() + file_path)
                );
                DocumentTree::build(parsed_object.dom_root)?
                    .print_in_chain(self.tree_trace_level)
                    .to_render_tree(style_sheets)?
                    .print_in_chain(self.tree_trace_level)
                    .to_box_tree(&self.draw_ctx)?
                    .print_in_chain(self.tree_trace_level)
                    .cleanup()?
                    .print_in_chain(self.tree_trace_level)
                    .layout(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)?
                    .print(self.tree_trace_level);
            }
        }

        Ok(())
    }

    /// Prints a CSS document as a style sheet.
    pub fn print_style_sheet(&self, css: &str) -> Result<()> {
        CssParser::new(&CssTokenizer::new(css).tokenize()?)
            .parse()?
            .print();
        Ok(())
    }
}
