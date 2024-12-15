use std::cell::RefCell;
use std::cmp::max_by;
use std::rc::Rc;

use anyhow::Result;
use gtk4::pango;
use regex::Regex;

use crate::renderer::layout::box_model::{LayoutBox, LayoutInfo};
use crate::renderer::style::property::{AbsoluteLengthUnit, CssValue, DisplayOutside, LengthUnit};
use crate::renderer::style::style_model::RenderNode;

#[derive(Debug)]
pub struct Text {
    pub style_node: Rc<RefCell<RenderNode>>,
    pub layout_info: LayoutInfo,
    pub draw_ctx: pango::Context,
}

impl LayoutBox for Text {
    fn layout(
        &mut self,
        containing_block_info: &LayoutInfo,
        parent_info: Option<LayoutInfo>,
        prev_sibling_info: Option<LayoutInfo>,
    ) {
        let orig_x = if let Some(prev_sibling_info) = prev_sibling_info {
            prev_sibling_info.pos.x + prev_sibling_info.size.width
        } else {
            parent_info.unwrap().pos.x
        };

        self.calc_used_values();
        self.calc_width_and_height(containing_block_info);
        self.calc_pos(containing_block_info, orig_x);
    }

    fn layout_children(&mut self, _: &LayoutInfo) {}
}

impl Text {
    /// Removes unnecessary whitespace from the text.
    /// https://developer.mozilla.org/en-US/docs/Web/API/Document_Object_Model/Whitespace
    pub fn trim_text(&mut self, is_first_child: bool, is_last_child: bool) -> Result<()> {
        let text = self
            .style_node
            .borrow()
            .node
            .borrow()
            .get_inside_text()
            .unwrap();

        let text = Regex::new(r"[ \t]*\n[ \t]*")?
            // 1. All spaces and tabs immediately before and after a line break are ignored.
            .replace_all(&text, "\n")
            // 2. All tab characters are converted to space characters.
            .replace("\t", " ")
            // 3. All line breaks are transformed to spaces.
            .replace("\n", " ");

        let text = Regex::new(r" +")?
            // 4. Any space immediately following another space (even across two separate inline elements) is ignored.
            .replace_all(&text, " ");

        let text = match self.style_node.borrow().get_display_type() {
            // 5. All spaces at the beginning and end of the block box are removed.
            DisplayOutside::Block => text.trim(),
            // 5'. Sequences of spaces at the beginning and end of an element are removed.
            DisplayOutside::Inline => {
                if is_first_child {
                    text.trim_start()
                } else if is_last_child {
                    text.trim_end()
                } else {
                    &text
                }
            }
        };

        self.style_node
            .borrow_mut()
            .node
            .borrow_mut()
            .set_inside_text(text);

        Ok(())
    }

    fn calc_used_values(&mut self) {
        [
            (
                self.layout_info.used_values.margin.left,
                &self.style_node.borrow().style.margin.left,
            ),
            (
                self.layout_info.used_values.margin.right,
                &self.style_node.borrow().style.margin.right,
            ),
            (
                self.layout_info.used_values.margin.top,
                &self.style_node.borrow().style.margin.top,
            ),
            (
                self.layout_info.used_values.margin.bottom,
                &self.style_node.borrow().style.margin.bottom,
            ),
        ]
        .iter_mut()
        .for_each(|(used_margin, comp_margin)| {
            *used_margin = match comp_margin {
                CssValue::Ident(v) if v == "auto" => 0.0,
                CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)) => {
                    *size
                }
                CssValue::Percentage(_) => unimplemented!(),
                _ => unreachable!(),
            };
        });
        self.layout_info.used_values.width = None;
    }

    fn calc_pos(&mut self, containing_block_info: &LayoutInfo, orig_x: f32) {
        self.layout_info.pos.x = orig_x
            + self.layout_info.used_values.margin.left
            + self.layout_info.used_values.border.left
            + self.layout_info.used_values.padding.left;
        self.layout_info.pos.y = containing_block_info.pos.y
            + self.layout_info.used_values.margin.top
            + self.layout_info.used_values.border.top
            + self.layout_info.used_values.padding.top;
    }

    fn calc_width_and_height(&mut self, containing_block_info: &LayoutInfo) {
        let CssValue::Length(font_size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)) =
            self.style_node.borrow().style.font_size.size
        else {
            unreachable!()
        };

        let font_family = self.style_node.borrow().style.font_family.to_name_list();
        let font_weight = &self.style_node.borrow().style.font_weight.to_name();
        let font_desc = pango::FontDescription::from_string(&format!(
            "{} {} {}px",
            font_family.join(", "),
            font_weight,
            font_size
        ));

        let max_line_width = self.wrap_text(&font_desc, containing_block_info);
        let height = self
            .style_node
            .borrow()
            .node
            .borrow()
            .get_inside_text()
            .unwrap()
            .split('\n')
            .map(|line| {
                let layout = pango::Layout::new(&self.draw_ctx);
                layout.set_font_description(Some(&font_desc));
                layout.set_text(line);
                layout.size().1 as f32 / pango::SCALE as f32
            })
            .sum::<f32>();

        self.layout_info.size.width = max_line_width as f32;
        self.layout_info.size.height = height;
    }

    /// Wraps the text by inserting line breaks at appropriate places to fit the width
    /// of the containing block, and returns the maximum line width and the number of lines.
    ///
    /// This implementation is quite simple and doesn't take into account the line box system
    /// and Unicode line-break rules. It also assumes that a text node is the only child of
    /// a block-level box (if it isn't, the line breaks won't work properly).
    /// https://drafts.csswg.org/css-text/#line-breaking
    fn wrap_text(
        &mut self,
        font_desc: &pango::FontDescription,
        containing_block_info: &LayoutInfo,
    ) -> f64 {
        let text = self
            .style_node
            .borrow()
            .node
            .borrow()
            .get_inside_text()
            .unwrap();
        let mut new_text = String::new();
        let mut curr_width = 0.0;
        let mut max_line_width = 0.0;

        let layout = pango::Layout::new(&self.draw_ctx);
        layout.set_font_description(Some(font_desc));
        layout.set_text(" ");
        let space_width = layout.size().0 as f64 / pango::SCALE as f64;

        text.split(' ').for_each(|word| {
            let layout = pango::Layout::new(&self.draw_ctx);
            layout.set_font_description(Some(font_desc));
            layout.set_text(word);
            let word_width = layout.size().0 as f64 / pango::SCALE as f64;
            curr_width += word_width;
            if curr_width as f32 >= containing_block_info.used_values.width.unwrap() {
                curr_width -= word_width;
                if new_text.ends_with(' ') {
                    new_text.pop();
                    curr_width -= space_width;
                }
                new_text.push_str(format!("\n{word} ").as_str());
                max_line_width =
                    max_by(max_line_width, curr_width, |a, b| a.partial_cmp(b).unwrap());
                curr_width = word_width;
            } else {
                new_text.push_str(format!("{word} ").as_str());
                curr_width += space_width;
            }
        });
        if new_text.ends_with(' ') {
            new_text.pop();
            curr_width -= space_width;
        }
        max_line_width = max_by(max_line_width, curr_width, |a, b| a.partial_cmp(b).unwrap());
        self.style_node
            .borrow()
            .node
            .borrow_mut()
            .set_inside_text(&new_text);

        max_line_width
    }
}
