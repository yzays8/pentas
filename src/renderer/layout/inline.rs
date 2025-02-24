use std::cell::RefCell;
use std::rc::Rc;

use crate::renderer::layout::text::Text;
use crate::renderer::layout::{BoxNode, BoxPosition, BoxSize, LayoutBox, LayoutInfo};
use crate::renderer::object::RenderObject;
use crate::renderer::style::RenderNode;
use crate::renderer::style::property::{CssValue, DisplayInside, DisplayOutside};

#[derive(Debug)]
pub struct InlineBox {
    pub style_node: Rc<RefCell<RenderNode>>,
    pub layout_info: LayoutInfo,
    pub children: Vec<Rc<RefCell<BoxNode>>>,
}

impl LayoutBox for InlineBox {
    fn layout(
        &mut self,
        containing_block_info: &LayoutInfo,
        _: Option<LayoutInfo>,
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

    fn layout_children(&mut self, containing_block_info: &LayoutInfo) {
        if self.children.is_empty() {
            return;
        }

        let is_every_child_inline = self
            .children
            .iter()
            .all(|child| matches!(*child.borrow(), BoxNode::InlineBox(_) | BoxNode::Text(_)));
        if !is_every_child_inline {
            unimplemented!(
                "Only inline-level boxes and text nodes are currently supported as children of a inline-level box."
            );
        }

        let mut inline_width = 0.0;
        let mut inline_max_height = 0.0;
        let mut prev_sib_info = None;

        for child in self.children.iter_mut() {
            // The containing block of an inline-level box is the nearest block-level ancestor box.
            // https://developer.mozilla.org/en-US/docs/Web/CSS/Containing_block
            // todo: Implement the line box system for simplification.
            child.borrow_mut().layout(
                containing_block_info,
                Some(self.layout_info.clone()),
                prev_sib_info,
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

    fn to_render_objects(&self, viewport_width: i32, viewport_height: i32) -> Vec<RenderObject> {
        let mut objects = Vec::new();

        for child in self.children.iter() {
            objects.extend(
                child
                    .borrow()
                    .to_render_objects(viewport_width, viewport_height),
            );
        }

        objects
    }
}

impl InlineBox {
    fn calc_used_values(&mut self) {
        let margin = self.style_node.borrow().style.margin.clone();
        let display = self.style_node.borrow().style.display.clone();

        if (display.outside, display.inside) != (DisplayOutside::Inline, DisplayInside::Flow) {
            unimplemented!("Only inline-level boxes in normal flow are currently supported.");
        }

        self.layout_info.used_values.margin.top = match margin.top {
            CssValue::Ident(v) if v == "auto" => 0.0,
            CssValue::Length(..) => margin.top.to_px().unwrap(),
            CssValue::Percentage(_) => unimplemented!(),
            _ => unreachable!(),
        };
        self.layout_info.used_values.margin.bottom = match margin.bottom {
            CssValue::Ident(v) if v == "auto" => 0.0,
            CssValue::Length(..) => margin.bottom.to_px().unwrap(),
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
        let start_x = if let (
            Some(BoxPosition { x: prev_x, .. }),
            Some(BoxSize {
                width: prev_width, ..
            }),
        ) = (&prev_sibling_pos, &prev_sibling_size)
        {
            prev_x + prev_width
        } else {
            containing_block_info.pos.x
        };

        // In inline-level boxes, margins are treated differently from block-level boxes.
        self.layout_info.pos.x = start_x
            + self.layout_info.used_values.margin.left
            + self.layout_info.used_values.border.left
            + self.layout_info.used_values.padding.left;
        self.layout_info.pos.y = containing_block_info.pos.y
            + self.layout_info.used_values.margin.top
            + self.layout_info.used_values.border.top
            + self.layout_info.used_values.padding.top;
    }
}
