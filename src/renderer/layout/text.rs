use std::cell::RefCell;
use std::cmp::max_by;
use std::rc::Rc;

use font_kit::family_name::FamilyName;
use font_kit::metrics::Metrics;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use crate::renderer::layout::box_model::{BoxPosition, LayoutInfo};
use crate::renderer::style::property::{AbsoluteLengthUnit, CssValue, LengthUnit};
use crate::renderer::style::style_model::RenderNode;

#[derive(Debug)]
pub struct Text {
    pub node: Rc<RefCell<RenderNode>>,
    pub layout_info: LayoutInfo,
}

impl Text {
    pub fn layout(
        &mut self,
        containing_block_info: &LayoutInfo,
        prev_sibling_info: Option<LayoutInfo>,
        parent_info: &BoxPosition,
    ) {
        let orig_x = if let Some(prev_sibling_info) = prev_sibling_info {
            prev_sibling_info.pos.x + prev_sibling_info.size.width
        } else {
            parent_info.x
        };

        self.calc_used_values();
        self.calc_width_and_height(containing_block_info);
        self.calc_pos(containing_block_info, orig_x);
    }

    fn calc_used_values(&mut self) {
        [
            (
                self.layout_info.used_values.margin.left,
                &self.node.borrow().style.margin.left,
            ),
            (
                self.layout_info.used_values.margin.right,
                &self.node.borrow().style.margin.right,
            ),
            (
                self.layout_info.used_values.margin.top,
                &self.node.borrow().style.margin.top,
            ),
            (
                self.layout_info.used_values.margin.bottom,
                &self.node.borrow().style.margin.bottom,
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
        self.layout_info.pos.x = self.layout_info.used_values.margin.left
            + self.layout_info.used_values.border.left
            + self.layout_info.used_values.padding.left
            + orig_x;
        self.layout_info.pos.y = self.layout_info.used_values.margin.top
            + self.layout_info.used_values.border.top
            + self.layout_info.used_values.padding.top
            + containing_block_info.pos.y;
    }

    fn calc_width_and_height(&mut self, containing_block_info: &LayoutInfo) {
        let CssValue::Length(font_size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)) =
            self.node.borrow().style.font_size.size
        else {
            unreachable!()
        };

        let font = SystemSource::new()
            .select_best_match(&[FamilyName::SansSerif], &Properties::new())
            .unwrap()
            .load()
            .unwrap();
        let text = self.node.borrow().node.borrow().get_inside_text().unwrap();
        let metrics: Metrics = font.metrics();
        let scale_factor = font_size / metrics.units_per_em as f32;
        let max_height = (metrics.ascent - metrics.descent) * scale_factor;

        // Wrap the text.
        // This implementation is quite simple and doesn't take into account the line box system
        // and Unicode line-break rules. It also assumes that a text node is the only child of
        // a block-level box (if it isn't, the line breaks won't work properly).
        // https://drafts.csswg.org/css-text/#line-breaking
        let mut new_text = String::new();
        let mut curr_width = 0.0;
        let mut max_line_width = 0.0;
        let space_width =
            font.advance(
                font.glyph_for_char(' ')
                    .unwrap_or(font.glyph_for_char('?').unwrap()),
            )
            .unwrap()
            .x() * scale_factor;
        let mut line_num = 1;
        text.split(' ').for_each(|word| {
            let word_width = word
                .chars()
                .map(|c| {
                    let glyph_id = font
                        .glyph_for_char(c)
                        .unwrap_or(font.glyph_for_char('?').unwrap());
                    let advance = font.advance(glyph_id);
                    advance.unwrap().x() * scale_factor
                })
                .sum::<f32>();
            curr_width += word_width;
            if containing_block_info.size.width < curr_width {
                // println!("{} {}", curr_width, containing_block_info.size.width);
                curr_width -= word_width;
                if new_text.ends_with(' ') {
                    new_text.pop();
                    curr_width -= space_width;
                }
                new_text.push_str(format!("\n{word} ").as_str());
                max_line_width =
                    max_by(max_line_width, curr_width, |a, b| a.partial_cmp(b).unwrap());
                curr_width = word_width;
                line_num += 1;
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
        self.node
            .borrow()
            .node
            .borrow_mut()
            .set_inside_text(&new_text);

        self.layout_info.size.width = max_line_width;
        self.layout_info.size.height = max_height * line_num as f32;
    }
}
