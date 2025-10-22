mod css;
mod html;
mod layout;
mod object;
mod style;

pub use self::{
    html::parser::HtmlParsedObject,
    object::{RenderObjsInfo, paint},
};

use gtk4::pango;

use self::{
    css::{CssParser, CssTokenizer, get_ua_style_sheet},
    html::{dom::DocumentTree, parser::HtmlParser},
};
use crate::{app::DumpLevel, error::Result, utils::PrintableTree as _};

#[derive(Debug, Default)]
pub struct Renderer {
    dump_level: DumpLevel,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_dump_level(&mut self, dump_level: DumpLevel) {
        self.dump_level = dump_level;
    }

    pub fn get_parsed_obj(&self, html: &str) -> Result<HtmlParsedObject> {
        HtmlParser::new(html).parse()
    }

    pub fn get_render_objs_info(
        &self,
        parsed_obj: HtmlParsedObject,
        text_ctx: &pango::Context,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<RenderObjsInfo> {
        let style_sheets = std::iter::once(get_ua_style_sheet()?)
            .chain(parsed_obj.style_sheets)
            .collect::<Vec<_>>();

        match self.dump_level {
            DumpLevel::Off => {
                let box_tree = DocumentTree::build(parsed_obj.dom_root)?
                    .to_render_tree(style_sheets, viewport_width, viewport_height)?
                    .to_box_tree(text_ctx)?
                    .cleanup()?
                    .layout(viewport_width, viewport_height)?;
                let (max_width, max_height) = box_tree.get_max_size();
                Ok(RenderObjsInfo {
                    objs: box_tree.to_render_objs(viewport_width, viewport_height),
                    max_width,
                    max_height,
                })
            }
            DumpLevel::All | DumpLevel::Debug => {
                let box_tree = DocumentTree::build(parsed_obj.dom_root)?
                    .print_in_chain(self.dump_level)
                    .to_render_tree(style_sheets, viewport_width, viewport_height)?
                    .print_in_chain(self.dump_level)
                    .to_box_tree(text_ctx)?
                    .print_in_chain(self.dump_level)
                    .cleanup()?
                    .print_in_chain(self.dump_level)
                    .layout(viewport_width, viewport_height)?
                    .print_in_chain(self.dump_level);
                let (max_width, max_height) = box_tree.get_max_size();
                Ok(RenderObjsInfo {
                    objs: box_tree.to_render_objs(viewport_width, viewport_height),
                    max_width,
                    max_height,
                })
            }
        }
    }

    /// Prints an HTML document as a box tree.
    pub fn print_box_tree(
        &self,
        html: &str,
        text_ctx: &pango::Context,
        window_size: (i32, i32),
    ) -> Result<()> {
        let parsed_obj = HtmlParser::new(html).parse()?;
        let style_sheets = std::iter::once(get_ua_style_sheet()?)
            .chain(parsed_obj.style_sheets)
            .collect::<Vec<_>>();

        match self.dump_level {
            DumpLevel::Off => {
                DocumentTree::build(parsed_obj.dom_root)?
                    .to_render_tree(style_sheets, window_size.0, window_size.1)?
                    .to_box_tree(text_ctx)?
                    .cleanup()?
                    .layout(window_size.0, window_size.1)?
                    .print(self.dump_level);
            }
            DumpLevel::All | DumpLevel::Debug => {
                DocumentTree::build(parsed_obj.dom_root)?
                    .print_in_chain(self.dump_level)
                    .to_render_tree(style_sheets, window_size.0, window_size.1)?
                    .print_in_chain(self.dump_level)
                    .to_box_tree(text_ctx)?
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
