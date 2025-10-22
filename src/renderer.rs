mod css;
mod html;
mod layout;
mod object;
mod style;

pub use self::{
    html::parser::ParsedObject,
    object::{RenderObject, RenderObjectsInfo},
};

use gtk4::pango;

use self::{
    css::{CssParser, CssTokenizer, get_ua_style_sheet},
    html::{dom::DocumentTree, parser::HtmlParser},
};
use crate::{app::DumpLevel, error::Result, utils::PrintableTree as _};

#[derive(Debug, Clone, Default)]
pub struct Renderer {
    draw_ctx: pango::Context,
    dump_level: DumpLevel,
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

    pub fn set_dump_level(&mut self, dump_level: DumpLevel) {
        self.dump_level = dump_level;
    }

    pub fn get_parsed_object(&self, html: &str) -> Result<ParsedObject> {
        HtmlParser::new(html).parse()
    }

    pub fn get_render_objects_info(
        &self,
        parsed_object: ParsedObject,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<RenderObjectsInfo> {
        let style_sheets = std::iter::once(get_ua_style_sheet()?)
            .chain(parsed_object.style_sheets)
            .collect::<Vec<_>>();

        match self.dump_level {
            DumpLevel::Off => {
                let box_tree = DocumentTree::build(parsed_object.dom_root)?
                    .to_render_tree(style_sheets, viewport_width, viewport_height)?
                    .to_box_tree(&self.draw_ctx)?
                    .cleanup()?
                    .layout(viewport_width, viewport_height)?;
                let (max_width, max_height) = box_tree.get_max_size();
                Ok(RenderObjectsInfo {
                    objects: box_tree.to_render_objects(viewport_width, viewport_height),
                    max_width,
                    max_height,
                })
            }
            DumpLevel::All | DumpLevel::Debug => {
                let box_tree = DocumentTree::build(parsed_object.dom_root)?
                    .print_in_chain(self.dump_level)
                    .to_render_tree(style_sheets, viewport_width, viewport_height)?
                    .print_in_chain(self.dump_level)
                    .to_box_tree(&self.draw_ctx)?
                    .print_in_chain(self.dump_level)
                    .cleanup()?
                    .print_in_chain(self.dump_level)
                    .layout(viewport_width, viewport_height)?
                    .print_in_chain(self.dump_level);
                let (max_width, max_height) = box_tree.get_max_size();
                Ok(RenderObjectsInfo {
                    objects: box_tree.to_render_objects(viewport_width, viewport_height),
                    max_width,
                    max_height,
                })
            }
        }
    }

    /// Prints an HTML document as a box tree.
    pub fn print_box_tree(&self, html: &str, window_size: (i32, i32)) -> Result<()> {
        let parsed_object = HtmlParser::new(html).parse()?;
        let style_sheets = std::iter::once(get_ua_style_sheet()?)
            .chain(parsed_object.style_sheets)
            .collect::<Vec<_>>();

        match self.dump_level {
            DumpLevel::Off => {
                DocumentTree::build(parsed_object.dom_root)?
                    .to_render_tree(style_sheets, window_size.0, window_size.1)?
                    .to_box_tree(&self.draw_ctx)?
                    .cleanup()?
                    .layout(window_size.0, window_size.1)?
                    .print(self.dump_level);
            }
            DumpLevel::All | DumpLevel::Debug => {
                DocumentTree::build(parsed_object.dom_root)?
                    .print_in_chain(self.dump_level)
                    .to_render_tree(style_sheets, window_size.0, window_size.1)?
                    .print_in_chain(self.dump_level)
                    .to_box_tree(&self.draw_ctx)?
                    .print_in_chain(self.dump_level)
                    .cleanup()?
                    .print_in_chain(self.dump_level)
                    .layout(window_size.0, window_size.1)?
                    .print(self.dump_level);
            }
        }

        Ok(())
    }

    pub fn print_style_sheet(&self, css: &str) -> Result<()> {
        CssParser::new(&CssTokenizer::new(css).tokenize()?)
            .parse()?
            .print();
        Ok(())
    }
}
