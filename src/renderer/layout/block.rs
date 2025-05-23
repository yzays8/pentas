use std::{cell::RefCell, rc::Rc};

use crate::renderer::{
    html::dom::{Element, NodeType},
    layout::{BoxNode, LayoutBox, LayoutInfo, inline::InlineBox, text::Text},
    object::{RenderObject, RenderRect},
    style::{
        ComputedStyle, RenderNode,
        property::{AbsoluteLengthUnit, CssValue, DisplayInside, DisplayOutside, LengthUnit},
    },
};

#[derive(Debug)]
pub struct BlockBox {
    pub style_node: Rc<RefCell<RenderNode>>,
    pub layout_info: LayoutInfo,
    pub children: Vec<Rc<RefCell<BoxNode>>>,
}

impl LayoutBox for BlockBox {
    fn layout(
        &mut self,
        containing_block_info: &LayoutInfo,
        _: Option<LayoutInfo>,
        prev_sibling_info: Option<LayoutInfo>,
    ) {
        self.calc_used_values(containing_block_info);
        // The margin of the box is not included in the width because it is outside the box.
        self.layout_info.size.width = self.layout_info.used_values.border.left
            + self.layout_info.used_values.padding.left
            + self.layout_info.used_values.width.unwrap()
            + self.layout_info.used_values.padding.right
            + self.layout_info.used_values.border.right;
        self.calc_pos(containing_block_info, prev_sibling_info);
        self.layout_children(containing_block_info);
    }

    fn layout_children(&mut self, _: &LayoutInfo) {
        let is_every_child_block = self.children.iter().all(|child| {
            matches!(
                *child.borrow(),
                BoxNode::BlockBox(_) | BoxNode::AnonymousBox(_)
            )
        });
        let is_every_child_inline = self
            .children
            .iter()
            .all(|child| matches!(*child.borrow(), BoxNode::InlineBox(_) | BoxNode::Text(_)));

        if is_every_child_block {
            let mut prev_sib_info = None;

            // If `height` is `auto`, the height of the box depends on whether the element
            // has any block-level children and whether it has padding or borders.
            // https://www.w3.org/TR/CSS22/visudet.html#normal-block
            for child in self.children.iter_mut() {
                child.borrow_mut().layout(
                    &self.layout_info,
                    Some(self.layout_info.clone()),
                    prev_sib_info.clone(),
                );

                let child_ref = child.borrow();
                let child_layout_info = match *child_ref {
                    BoxNode::BlockBox(BlockBox {
                        ref layout_info, ..
                    })
                    | BoxNode::AnonymousBox(AnonymousBox {
                        ref layout_info, ..
                    }) => layout_info,
                    _ => unreachable!(),
                };

                // This is where the margin collapse happens, which is tricky. This
                // implementation is quite simple and does not cover complex cases.
                // https://www.w3.org/TR/CSS22/box.html#collapsing-margins
                // https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_box_model/Mastering_margin_collapsing
                if let Some(info) = &prev_sib_info {
                    if child_layout_info.used_values.margin.top < info.used_values.margin.bottom {
                        self.layout_info.size.height +=
                            child_layout_info.get_expanded_size().height
                                - child_layout_info.used_values.margin.top;
                    } else {
                        self.layout_info.size.height +=
                            child_layout_info.get_expanded_size().height
                                - info.used_values.margin.bottom;
                    }
                } else {
                    self.layout_info.size.height += child_layout_info.get_expanded_size().height;
                }

                prev_sib_info = Some(child_layout_info.clone());
            }

            // The margin of the box is not included in the height because it is outside the box.
            self.layout_info.size.height += self.layout_info.used_values.padding.top
                + self.layout_info.used_values.border.top
                + self.layout_info.used_values.padding.bottom
                + self.layout_info.used_values.border.bottom;
        } else if is_every_child_inline {
            let mut inline_max_height = 0.0;
            let mut prev_sib_info = None;

            for child in self.children.iter_mut() {
                child.borrow_mut().layout(
                    &self.layout_info,
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

                let ch_exp_height = child_layout_info.get_expanded_size().height;
                if inline_max_height < ch_exp_height {
                    inline_max_height = ch_exp_height;
                }
                prev_sib_info = Some(child_layout_info.clone());
            }

            // If parent is a block-level box and children are inline-level boxes, the parent's width
            // is defined by the parent itself (so the width is not determined here by the children).

            // The margin of the box is not included in the height because it is outside the box.
            self.layout_info.size.height = self.layout_info.used_values.border.top
                + self.layout_info.used_values.padding.top
                + inline_max_height
                + self.layout_info.used_values.padding.bottom
                + self.layout_info.used_values.border.bottom;
        } else if !self.children.is_empty() {
            unreachable!()
        }

        // If `height` is not `auto`, the height of the box is the value of `height`.
        if let CssValue::Length(height, _) = self.style_node.borrow().style.height.size {
            self.layout_info.size.height = self.layout_info.used_values.border.top
                + self.layout_info.used_values.padding.top
                + height
                + self.layout_info.used_values.padding.bottom
                + self.layout_info.used_values.border.bottom;
        }
    }

    fn to_render_objects(&self, viewport_width: i32, viewport_height: i32) -> Vec<RenderObject> {
        let mut objects = Vec::new();

        let (r, g, b, a) = self
            .style_node
            .borrow()
            .style
            .background_color
            .to_rgba()
            .unwrap();
        let border_radius = self
            .style_node
            .borrow()
            .style
            .border_radius
            .to_px()
            .unwrap();

        // The style of the body element is applied to the whole viewport.
        let is_body = if let NodeType::Element(Element { tag_name: n, .. }) =
            &self.style_node.borrow().dom_node.borrow().node_type
        {
            n == "body"
        } else {
            false
        };

        // Draw the rectangle only if the background color is not transparent.
        if a != 0.0 {
            objects.push(RenderObject::Rect(RenderRect {
                x: self.layout_info.pos.x as f64,
                y: self.layout_info.pos.y as f64,
                width: if is_body {
                    viewport_width as f64
                } else {
                    self.layout_info.size.width as f64
                },
                height: if is_body {
                    viewport_height as f64
                } else {
                    self.layout_info.size.height as f64
                },
                color: (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0),
                border_radius,
            }));
        }
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

impl BlockBox {
    fn calc_used_values(&mut self, containing_block_info: &LayoutInfo) {
        let (width, margin, display) = (
            self.style_node.borrow().style.width.clone(),
            self.style_node.borrow().style.margin.clone(),
            self.style_node.borrow().style.display.clone(),
        );
        let mut margin_left = margin.left;
        let mut margin_right = margin.right;

        match (display.outside, display.inside) {
            // Block-level, non-replaced elements in normal flow
            // https://www.w3.org/TR/CSS22/visudet.html#blockwidth
            (DisplayOutside::Block, DisplayInside::Flow) => {
                let sum = [&width.size, &margin_left, &margin_right]
                    .iter()
                    .map(|v| match v {
                        CssValue::Ident(v) if v == "auto" => 0.0,
                        CssValue::Length(..) => v.to_px().unwrap(),
                        CssValue::Percentage(_) => unimplemented!(),
                        _ => unreachable!(),
                    })
                    .sum::<f32>()
                    + self.layout_info.used_values.padding.left
                    + self.layout_info.used_values.padding.right
                    + self.layout_info.used_values.border.left
                    + self.layout_info.used_values.border.right;

                let leeway = containing_block_info.size.width
                    - containing_block_info.used_values.padding.left
                    - containing_block_info.used_values.padding.right
                    - containing_block_info.used_values.border.left
                    - containing_block_info.used_values.border.right
                    - sum;

                if (width.size != CssValue::Ident("auto".to_string())) && (leeway < 0.0) {
                    if margin_left == CssValue::Ident("auto".to_string()) {
                        margin_left = CssValue::Length(
                            0.0,
                            LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                        );
                    }
                    if margin_right == CssValue::Ident("auto".to_string()) {
                        margin_right = CssValue::Length(
                            0.0,
                            LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                        );
                    }
                }

                let is_width_auto = width.size == CssValue::Ident("auto".to_string());
                let is_margin_left_auto = margin_left == CssValue::Ident("auto".to_string());
                let is_margin_right_auto = margin_right == CssValue::Ident("auto".to_string());

                let (width_px, margin_left_px, margin_right_px) =
                    match (is_width_auto, is_margin_left_auto, is_margin_right_auto) {
                        (false, false, false) => {
                            // Assume that the `direction` property of the containing block is `ltr`.
                            (
                                width.size.to_px().unwrap(),
                                margin_left.to_px().unwrap(),
                                margin_right.to_px().unwrap() + leeway,
                            )
                        }
                        (false, true, true) => {
                            (width.size.to_px().unwrap(), leeway / 2.0, leeway / 2.0)
                        }
                        (true, _, _) => {
                            let margin_left_px = match margin_left {
                                CssValue::Ident(v) if v == "auto" => 0.0,
                                CssValue::Length(..) => margin_left.to_px().unwrap(),
                                CssValue::Percentage(_) => unimplemented!(),
                                _ => unreachable!(),
                            };
                            let margin_right_px = match margin_right {
                                CssValue::Ident(v) if v == "auto" => 0.0,
                                CssValue::Length(..) => margin_right.to_px().unwrap(),
                                CssValue::Percentage(_) => unimplemented!(),
                                _ => unreachable!(),
                            };

                            if leeway >= 0.0 {
                                (leeway, margin_left_px, margin_right_px)
                            } else {
                                (0.0, margin_left_px, margin_right_px + leeway)
                            }
                        }
                        _ => unimplemented!(),
                    };

                self.layout_info.used_values.width = Some(width_px);
                self.layout_info.used_values.margin.left = margin_left_px;
                self.layout_info.used_values.margin.right = margin_right_px;
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
            }

            _ => unimplemented!("Currently, only block-level boxes in normal flow are supported."),
        }
    }

    fn calc_pos(
        &mut self,
        containing_block_info: &LayoutInfo,
        prev_sibling_info: Option<LayoutInfo>,
    ) {
        // The value of x and y takes into account the border and padding of the box
        // but not the margin, because that's the space outside the box.

        self.layout_info.pos.x = containing_block_info.pos.x
            + containing_block_info.used_values.padding.left
            + self.layout_info.used_values.margin.left
            + self.layout_info.used_values.border.left;

        // This is where the margin collapse happens, which is tricky. This
        // implementation is quite simple and does not cover complex cases.
        // https://www.w3.org/TR/CSS22/box.html#collapsing-margins
        // https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_box_model/Mastering_margin_collapsing
        self.layout_info.pos.y = self.layout_info.used_values.border.top
            + if let Some(info) = prev_sibling_info {
                if self.layout_info.used_values.margin.top < info.used_values.margin.bottom {
                    info.get_expanded_pos().y + info.get_expanded_size().height
                } else {
                    self.layout_info.used_values.margin.top
                        + info.get_expanded_pos().y
                        + info.get_expanded_size().height
                        - info.used_values.margin.bottom
                }
            } else {
                containing_block_info.pos.y
                    + containing_block_info.used_values.padding.top
                    + self.layout_info.used_values.margin.top
            };
    }
}

#[derive(Debug)]
pub struct AnonymousBox {
    pub style: Box<ComputedStyle>,
    pub layout_info: LayoutInfo,
    pub children: Vec<Rc<RefCell<BoxNode>>>,
}

impl LayoutBox for AnonymousBox {
    fn layout(
        &mut self,
        containing_block_info: &LayoutInfo,
        _: Option<LayoutInfo>,
        prev_sibling_info: Option<LayoutInfo>,
    ) {
        self.calc_used_values(containing_block_info);
        self.layout_info.size.width = containing_block_info.size.width;
        self.calc_pos(containing_block_info, prev_sibling_info);
        self.layout_children(containing_block_info);
    }

    fn layout_children(&mut self, _: &LayoutInfo) {
        if self.children.is_empty() {
            unreachable!()
        }
        let is_every_child_inline = self
            .children
            .iter()
            .all(|child| matches!(*child.borrow(), BoxNode::InlineBox(_) | BoxNode::Text(_)));
        if !is_every_child_inline {
            unreachable!(
                "AnonymousBox currently only supports inline-level boxes and text nodes as children."
            );
        }

        let mut inline_max_height = 0.0;
        let mut prev_sib_info = None;

        // Assume that all children are inline-level boxes or text nodes.
        for child in self.children.iter_mut() {
            // The containing block of an inline-level box is the nearest block-level ancestor box.
            child.borrow_mut().layout(
                &self.layout_info,
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

            let ch_exp_height = child_layout_info.get_expanded_size().height;
            if inline_max_height < ch_exp_height {
                inline_max_height = ch_exp_height;
            }
            prev_sib_info = Some(child_layout_info.clone());
        }

        // If parent is a block-level box and children are inline-level boxes, the parent's width
        // is defined by the parent itself (so the width is not determined here by the children).
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

impl AnonymousBox {
    pub fn calc_used_values(&mut self, containing_block_info: &LayoutInfo) {
        self.layout_info.used_values.width = Some(containing_block_info.size.width);
        self.layout_info.used_values.margin.top = 0.0;
        self.layout_info.used_values.margin.right = 0.0;
        self.layout_info.used_values.margin.bottom = 0.0;
        self.layout_info.used_values.margin.left = 0.0;
    }

    /// https://www.w3.org/TR/CSS22/visudet.html#normal-block
    pub fn calc_pos(
        &mut self,
        containing_block_info: &LayoutInfo,
        prev_sibling_info: Option<LayoutInfo>,
    ) {
        self.layout_info.pos.x = containing_block_info.pos.x;
        self.layout_info.pos.y = if let Some(prev_sib_info) = prev_sibling_info {
            prev_sib_info.get_expanded_pos().y + prev_sib_info.get_expanded_size().height
        } else {
            containing_block_info.pos.y
        };
    }
}
