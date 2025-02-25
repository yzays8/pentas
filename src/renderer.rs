mod css;
mod html;
mod layout;
mod object;
mod style;

pub use self::object::{RenderObject, RenderObjectsInfo};

use anyhow::Result;
use gtk4::pango;

use self::{
    css::{CssParser, CssTokenizer, get_ua_style_sheet},
    html::{dom::DocumentTree, parser::HtmlParser, token::HtmlTokenizer},
};
use crate::{
    app::TreeTraceLevel,
    ui::{DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH},
    utils::PrintableTree as _,
};

#[derive(Debug, Clone, Default)]
pub struct Renderer {
    draw_ctx: pango::Context,
    trace_level: TreeTraceLevel,
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

    pub fn set_trace_level(&mut self, tree_trace_level: TreeTraceLevel) {
        self.trace_level = tree_trace_level;
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

        match self.trace_level {
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
                    .print_in_chain(self.trace_level)
                    .to_render_tree(style_sheets)?
                    .print_in_chain(self.trace_level)
                    .to_box_tree(&self.draw_ctx)?
                    .print_in_chain(self.trace_level)
                    .cleanup()?
                    .print_in_chain(self.trace_level)
                    .layout(viewport_width, viewport_height)?
                    .print_in_chain(self.trace_level);
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

        match self.trace_level {
            TreeTraceLevel::Silent => {
                DocumentTree::build(parsed_object.dom_root)?
                    .to_render_tree(style_sheets)?
                    .to_box_tree(&self.draw_ctx)?
                    .cleanup()?
                    .layout(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)?
                    .print(self.trace_level);
            }
            TreeTraceLevel::Normal | TreeTraceLevel::Debug => {
                println!(
                    "Title: {}\n",
                    parsed_object
                        .title
                        .unwrap_or_else(|| "file://".to_string() + file_path)
                );
                DocumentTree::build(parsed_object.dom_root)?
                    .print_in_chain(self.trace_level)
                    .to_render_tree(style_sheets)?
                    .print_in_chain(self.trace_level)
                    .to_box_tree(&self.draw_ctx)?
                    .print_in_chain(self.trace_level)
                    .cleanup()?
                    .print_in_chain(self.trace_level)
                    .layout(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)?
                    .print(self.trace_level);
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
