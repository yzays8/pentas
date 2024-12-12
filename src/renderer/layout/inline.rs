use std::cell::RefCell;
use std::rc::Rc;

use crate::renderer::layout::box_model::{BoxNode, BoxPosition, BoxSize, LayoutInfo};
use crate::renderer::layout::text::Text;
use crate::renderer::style::property::display::{DisplayInside, DisplayOutside};
use crate::renderer::style::property::{AbsoluteLengthUnit, CssValue, LengthUnit};
use crate::renderer::style::style_model::RenderNode;

#[derive(Debug)]
pub struct InlineBox {
    pub node: Rc<RefCell<RenderNode>>,
    pub layout_info: LayoutInfo,
    pub child_nodes: Vec<Rc<RefCell<BoxNode>>>,
}

impl InlineBox {
    pub fn layout(
        &mut self,
        containing_block_info: &LayoutInfo,
        prev_sibling_info: Option<LayoutInfo>,
    ) {
        let (prev_sibling_pos, prev_sibling_size) =
            if let Some(prev_sibling_info) = prev_sibling_info {
                (
                    Some(prev_sibling_info.get_expanded_pos()),
                    Some(prev_sibling_info.get_expanded_size()),
                )
            } else {
                (None, None)
            };
        self.calc_used_values();
        self.calc_pos(containing_block_info, prev_sibling_pos, prev_sibling_size);
        self.layout_children(containing_block_info);
    }

    fn calc_used_values(&mut self) {
        let margin = self.node.borrow().style.margin.clone();
        let display = self.node.borrow().style.display.clone();

        if (display.outside, display.inside) != (DisplayOutside::Inline, DisplayInside::Flow) {
            unimplemented!("Only inline-level boxes in normal flow are currently supported.");
        }

        self.layout_info.used_values.margin.top = match margin.top {
            CssValue::Ident(v) if v == "auto" => 0.0,
            CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)) => size,
            CssValue::Percentage(_) => unimplemented!(),
            _ => unreachable!(),
        };
        self.layout_info.used_values.margin.bottom = match margin.bottom {
            CssValue::Ident(v) if v == "auto" => 0.0,
            CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)) => size,
            CssValue::Percentage(_) => unimplemented!(),
            _ => unreachable!(),
        };
        self.layout_info.used_values.width = None;
    }

    fn calc_pos(
        &mut self,
        containing_block_info: &LayoutInfo,
        prev_sibling_pos: Option<BoxPosition>,
        prev_sibling_size: Option<BoxSize>,
    ) {
        // todo: When the inline-level box runs over the edge of the line box, it is split into several boxes.
        // https://www.w3.org/TR/CSS22/visuren.html#inline-formatting
        self.layout_info.pos.x = self.layout_info.used_values.margin.left
            + self.layout_info.used_values.border.left
            + self.layout_info.used_values.padding.left
            + if let (Some(BoxPosition { x, .. }), Some(BoxSize { width, .. })) =
                (&prev_sibling_pos, &prev_sibling_size)
            {
                x + width
            } else {
                containing_block_info.pos.x
            };
        self.layout_info.pos.y = self.layout_info.used_values.margin.top
            + self.layout_info.used_values.border.top
            + self.layout_info.used_values.padding.top
            + containing_block_info.pos.y;
    }

    fn layout_children(&mut self, containing_block_info: &LayoutInfo) {
        if self.child_nodes.is_empty() {
            return;
        }

        let is_every_child_inline = self
            .child_nodes
            .iter()
            .all(|child| matches!(*child.borrow(), BoxNode::InlineBox(_) | BoxNode::Text(_)));
        if !is_every_child_inline {
            unimplemented!("Only inline-level boxes and text nodes are currently supported as children of a inline-level box.");
        }

        let mut inline_width = 0.0;
        let mut inline_max_height = 0.0;
        let mut prev_sib_info = None;

        for child in self.child_nodes.iter_mut() {
            // The containing block of an inline-level box is the nearest block-level ancestor box.
            // todo: Implement the line box system for simplification.
            child.borrow_mut().layout(
                containing_block_info,
                prev_sib_info,
                Some(self.layout_info.pos),
            );

            let child_ref = child.borrow();
            let child_layout_info = match *child_ref {
                BoxNode::InlineBox(InlineBox {
                    ref layout_info, ..
                })
                | BoxNode::Text(Text {
                    ref layout_info, ..
                }) => layout_info,
                _ => unreachable!(),
            };

            let ch_exp_width = child_layout_info.get_expanded_size().width;
            let ch_exp_height = child_layout_info.get_expanded_size().height;
            inline_width += ch_exp_width;
            if inline_max_height < ch_exp_height {
                inline_max_height = ch_exp_height;
            }
            prev_sib_info = Some(child_layout_info.clone());
        }

        // If parent is an inline-level box and children are inline-level boxes,
        // the parent's width is the sum of the children's widths.
        self.layout_info.size.width = inline_width;

        self.layout_info.size.height = inline_max_height;
    }
}
