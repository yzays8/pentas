use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use anyhow::{bail, ensure, Context, Ok, Result};
use font_kit::family_name::FamilyName;
use font_kit::metrics::Metrics;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use regex::Regex;

use crate::renderer::css::dtype::{AbsoluteLengthUnit, CssValue, LengthUnit};
use crate::renderer::css::property::display::{DisplayInside, DisplayOutside};
use crate::renderer::html::dom::{Element, NodeType};
use crate::renderer::style::{ComputedValues, RenderNode, RenderTree};

/// https://www.w3.org/TR/css-display-3/#box-tree
#[derive(Debug)]
pub struct BoxTree {
    root: Rc<RefCell<BoxNode>>,
}

impl BoxTree {
    pub fn build(render_tree: &RenderTree) -> Result<Self> {
        ensure!(
            render_tree.root.borrow().node.borrow().node_type == NodeType::Document,
            "The root node of the render tree must be a document node."
        );

        // https://www.w3.org/TR/css-display-3/#root-element
        let mut root = None;
        for child in render_tree.root.borrow().child_nodes.iter() {
            if let NodeType::Element(Element { tag_name: n, .. }) =
                &child.borrow().node.borrow().node_type
            {
                if n == "html" {
                    root = Some(Rc::clone(child));
                }
            }
        }
        ensure!(
            root.is_some(),
            "The element at the root box of the box tree must be an HTML element node."
        );

        Ok(Self {
            root: Rc::new(RefCell::new(
                BoxNode::build(root.unwrap()).context("Failed to build box tree")?,
            )),
        })
    }

    pub fn layout(&mut self, viewport_width: f32) -> Result<&mut Self> {
        self.root.borrow_mut().layout(
            &LayoutInfo {
                size: BoxSize {
                    // https://www.w3.org/TR/CSS22/visuren.html#viewport
                    width: viewport_width,
                    height: 0.0,
                },
                pos: BoxPosition { x: 0.0, y: 0.0 },
                used_values: UsedValues::default(),
            },
            None,
            None,
            None,
        )?;
        Ok(self)
    }

    pub fn print(&self) {
        println!("{}", self);
    }

    /// Removes unnecessary whitespace from all text nodes in the tree.
    /// https://developer.mozilla.org/en-US/docs/Web/API/Document_Object_Model/Whitespace
    pub fn clean_up(&mut self) -> Result<&mut Self> {
        Ok(self.remove_whitespace()?.remove_empty_anonymous_boxes())
    }

    fn remove_whitespace(&mut self) -> Result<&mut Self> {
        fn helper(
            node: &mut Rc<RefCell<BoxNode>>,
            is_first_child: bool,
            is_last_child: bool,
        ) -> Result<()> {
            if let BoxKind::Text(tnode) = &node.borrow().box_kind {
                let text = tnode.borrow().node.borrow().get_inside_text().unwrap();

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

                let text = match tnode.borrow().get_display_type() {
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

                tnode.borrow_mut().node.borrow_mut().node_type = NodeType::Text(text.to_string());
            }

            Ok(())
        }

        fn traverse_and_remove_whitespace(node: &mut Rc<RefCell<BoxNode>>) -> Result<()> {
            let child_num = node.borrow().child_nodes.len();
            let mut remove_list: Vec<usize> = vec![];
            for (i, child) in node.borrow_mut().child_nodes.iter_mut().enumerate() {
                helper(child, i == 0, i == child_num - 1)?;
                if let BoxKind::Text(tnode) = &child.borrow().box_kind {
                    if tnode.borrow().node.borrow().node_type == NodeType::Text("".to_string()) {
                        remove_list.push(i);
                    }
                }
                if !remove_list.contains(&i) {
                    traverse_and_remove_whitespace(child)?;
                }
            }
            for i in remove_list.iter().rev() {
                node.borrow_mut().child_nodes.remove(*i);
            }

            Ok(())
        }

        traverse_and_remove_whitespace(&mut self.root)?;
        Ok(self)
    }

    fn remove_empty_anonymous_boxes(&mut self) -> &mut Self {
        fn helper(node: &mut Rc<RefCell<BoxNode>>) {
            let mut remove_list: Vec<usize> = vec![];
            for (i, child) in node.borrow_mut().child_nodes.iter_mut().enumerate() {
                if let BoxKind::Anonymous(_) = child.borrow().box_kind {
                    if child.borrow().child_nodes.is_empty() {
                        remove_list.push(i);
                    }
                }
                // If the child is not removed as an empty anonymous box, recursively check its children.
                if !remove_list.contains(&i) {
                    helper(child);
                }
            }
            for i in remove_list.iter().rev() {
                node.borrow_mut().child_nodes.remove(*i);
            }
        }

        helper(&mut self.root);
        self
    }
}

impl fmt::Display for BoxTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn construct_node_view(
            node_tree: &mut String,
            node: &Rc<RefCell<BoxNode>>,
            current_depth: usize,
            is_last_child: bool,
            mut exclude_branches: Vec<usize>,
        ) {
            if is_last_child {
                exclude_branches.push(current_depth);
            }
            let mut indent_and_branches = String::new();
            for i in 0..current_depth {
                if exclude_branches.contains(&i) {
                    indent_and_branches.push_str("  ");
                } else {
                    indent_and_branches.push_str("│ ");
                }
            }
            indent_and_branches.push_str(if is_last_child { "└─" } else { "├─" });
            node_tree.push_str(&format!("{}{}\n", indent_and_branches, node.borrow()));
            let children_num = node.borrow().child_nodes.len();
            for (i, child) in node.borrow().child_nodes.iter().enumerate() {
                construct_node_view(
                    node_tree,
                    child,
                    current_depth + 1,
                    i == children_num - 1,
                    exclude_branches.clone(),
                );
            }
        }
        let mut node_tree = String::new();
        construct_node_view(&mut node_tree, &self.root, 0, true, vec![]);
        node_tree.pop(); // Remove the last newline character
        write!(f, "{}", node_tree)
    }
}

/// Generated box type from the `display` property.
#[derive(Debug)]
pub enum BoxKind {
    /// https://www.w3.org/TR/css-display-3/#block-box
    Block(Rc<RefCell<RenderNode>>),

    /// https://www.w3.org/TR/css-display-3/#inline-box
    Inline(Rc<RefCell<RenderNode>>),

    /// https://www.w3.org/TR/css-display-3/#css-text-sequence
    Text(Rc<RefCell<RenderNode>>),

    /// https://www.w3.org/TR/css-display-3/#anonymous
    Anonymous(Box<ComputedValues>),
}

/// Calculated width, height, and position of the BoxNode and its `used values` of the `width`, `margin`, `padding`, and `border` properties.
#[derive(Debug, Default)]
pub struct LayoutInfo {
    pub size: BoxSize,
    pub pos: BoxPosition,
    pub used_values: UsedValues,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct BoxSize {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct BoxPosition {
    pub x: f32,
    pub y: f32,
}

/// Used values for the `width`, `margin`, `padding`, and `border` properties.
#[derive(Debug, Default)]
pub struct UsedValues {
    /// The `width` property won't be applied to non-replaced inline elements.
    pub width: Option<f32>,
    pub margin: Edge,
    pub padding: Edge,
    pub border: Edge,
}

#[derive(Debug, Default)]
pub struct Edge {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug)]
pub struct BoxNode {
    pub box_kind: BoxKind,
    pub layout_info: LayoutInfo,
    pub child_nodes: Vec<Rc<RefCell<Self>>>,
}

// todo: Set correct layout information here
impl BoxNode {
    pub fn build(node: Rc<RefCell<RenderNode>>) -> Option<Self> {
        match node.borrow().node.borrow().node_type {
            NodeType::Document | NodeType::Comment(_) | NodeType::DocumentType(_) => return None,
            NodeType::Text(_) => {
                return Some(Self {
                    box_kind: BoxKind::Text(Rc::clone(&node)),
                    layout_info: LayoutInfo::default(),
                    child_nodes: vec![],
                })
            }
            _ => {}
        }

        // Create box nodes for the children of the current node.
        let mut children: Vec<Rc<RefCell<BoxNode>>> = Vec::new();
        let mut i = 0;
        while i < node.borrow().child_nodes.len() {
            match node.borrow().child_nodes[i].borrow().get_display_type() {
                DisplayOutside::Block => {
                    if node.borrow().get_display_type() == DisplayOutside::Inline {
                        // todo: It is tricky to handle block-level boxes within an inline box.
                        // https://www.w3.org/TR/CSS22/visuren.html#anonymous-block-level
                        // https://github.com/w3c/csswg-drafts/issues/1477
                        unimplemented!(
                            "Block-level boxes within an inline box are not yet supported: {}",
                            node.borrow().node.borrow().node_type
                        );
                    }

                    let child = Self::build(Rc::clone(&node.borrow().child_nodes[i]));
                    if let Some(child) = child {
                        children.push(Rc::new(RefCell::new(child)));
                    }
                }
                DisplayOutside::Inline => {
                    if (node.borrow().get_display_type() == DisplayOutside::Block)
                        && (node.borrow().child_nodes.len() > 1)
                    {
                        // Inline-level contents in block formatting context are wrapped in anonymous boxes.
                        let mut anon_box = BoxNode {
                            box_kind: BoxKind::Anonymous(Box::new(node.borrow().style.clone())),
                            layout_info: LayoutInfo::default(),
                            child_nodes: vec![],
                        };

                        // If there are successive inline-level contents, they are wrapped in the same anonymous box.
                        // https://www.w3.org/TR/css-inline-3/#root-inline-box
                        while i < node.borrow().child_nodes.len()
                            && node.borrow().child_nodes[i].borrow().get_display_type()
                                == DisplayOutside::Inline
                        {
                            let child = Self::build(Rc::clone(&node.borrow().child_nodes[i]));
                            if let Some(child) = child {
                                anon_box.child_nodes.push(Rc::new(RefCell::new(child)));
                            }
                            i += 1;
                        }
                        i -= 1;
                        children.push(Rc::new(RefCell::new(anon_box)));
                    } else {
                        let child = Self::build(Rc::clone(&node.borrow().child_nodes[i]));
                        if let Some(child) = child {
                            children.push(Rc::new(RefCell::new(child)));
                        }
                    }
                }
            }

            i += 1;
        }

        // Set the used values for the padding and border properties.
        // The margin property is set later because it needs to be resolved if an `auto` value is set.
        let padding = if let Some(padding_prop) = &node.borrow().style.padding {
            let (
                CssValue::Length(top_px, _),
                CssValue::Length(right_px, _),
                CssValue::Length(bottom_px, _),
                CssValue::Length(left_px, _),
            ) = (
                &padding_prop.top,
                &padding_prop.right,
                &padding_prop.bottom,
                &padding_prop.left,
            )
            else {
                unimplemented!();
            };
            Edge {
                top: *top_px,
                right: *right_px,
                bottom: *bottom_px,
                left: *left_px,
            }
        } else {
            unreachable!()
        };
        let border = if let Some(border_prop) = &node.borrow().style.border {
            let (
                CssValue::Length(top_px, _),
                CssValue::Length(right_px, _),
                CssValue::Length(bottom_px, _),
                CssValue::Length(left_px, _),
            ) = (
                &border_prop.border_width.top,
                &border_prop.border_width.right,
                &border_prop.border_width.bottom,
                &border_prop.border_width.left,
            )
            else {
                unreachable!();
            };
            Edge {
                top: *top_px,
                right: *right_px,
                bottom: *bottom_px,
                left: *left_px,
            }
        } else {
            unreachable!();
        };

        Some(Self {
            box_kind: match node.borrow().get_display_type() {
                DisplayOutside::Block => BoxKind::Block(Rc::clone(&node)),
                DisplayOutside::Inline => BoxKind::Inline(Rc::clone(&node)),
            },
            layout_info: LayoutInfo {
                used_values: UsedValues {
                    padding,
                    border,
                    ..Default::default()
                },
                ..Default::default()
            },
            child_nodes: children,
        })
    }

    /// Returns the size of the box after applying the margin, padding, and border properties.
    pub fn get_expanded_size(&self) -> BoxSize {
        BoxSize {
            width: self.layout_info.size.width
                + self.layout_info.used_values.margin.left
                + self.layout_info.used_values.margin.right
                + self.layout_info.used_values.padding.left
                + self.layout_info.used_values.padding.right
                + self.layout_info.used_values.border.left
                + self.layout_info.used_values.border.right,
            height: self.layout_info.size.height
                + self.layout_info.used_values.margin.top
                + self.layout_info.used_values.margin.bottom
                + self.layout_info.used_values.padding.top
                + self.layout_info.used_values.padding.bottom
                + self.layout_info.used_values.border.top
                + self.layout_info.used_values.border.bottom,
        }
    }

    /// Returns the position of the box after applying the margin, padding, and border properties.
    pub fn get_expanded_pos(&self) -> BoxPosition {
        BoxPosition {
            x: self.layout_info.pos.x
                - self.layout_info.used_values.margin.left
                - self.layout_info.used_values.padding.left
                - self.layout_info.used_values.border.left,
            y: self.layout_info.pos.y
                - self.layout_info.used_values.margin.top
                - self.layout_info.used_values.padding.top
                - self.layout_info.used_values.border.top,
        }
    }

    /// Sets the width, height, position, and used values for some properties of the box and its children.
    pub fn layout(
        &mut self,
        containing_block_info: &LayoutInfo,
        containing_block_style: Option<&ComputedValues>,
        prev_sibling_pos: Option<BoxPosition>,
        prev_sibling_size: Option<BoxSize>,
    ) -> Result<&mut Self> {
        match &self.box_kind {
            BoxKind::Block(_) | BoxKind::Anonymous(_) => {
                self.layout_block(containing_block_info, prev_sibling_pos, prev_sibling_size)?;
            }
            BoxKind::Inline(_) => {
                self.layout_inline(containing_block_info, prev_sibling_pos, prev_sibling_size)?;
            }
            BoxKind::Text(_) => self.layout_text(
                containing_block_info,
                containing_block_style,
                prev_sibling_pos,
                prev_sibling_size,
            ),
        }
        Ok(self)
    }

    fn layout_block(
        &mut self,
        containing_block_info: &LayoutInfo,
        prev_sibling_pos: Option<BoxPosition>,
        prev_sibling_size: Option<BoxSize>,
    ) -> Result<()> {
        self.calc_used_values_for_block(containing_block_info)?;
        if let BoxKind::Anonymous(_) = &self.box_kind {
            self.layout_info.size.width = containing_block_info.size.width;
        } else {
            self.layout_info.size.width = self.layout_info.used_values.width.unwrap();
        }
        self.calc_block_pos(containing_block_info, prev_sibling_pos, prev_sibling_size)?;
        self.layout_children()?;
        Ok(())
    }

    fn layout_inline(
        &mut self,
        containing_block_info: &LayoutInfo,
        prev_sibling_pos: Option<BoxPosition>,
        prev_sibling_size: Option<BoxSize>,
    ) -> Result<()> {
        self.calc_used_values_for_inline()?;
        self.calc_inline_pos(containing_block_info, prev_sibling_pos, prev_sibling_size)?;
        self.layout_children()?;
        Ok(())
    }

    fn layout_text(
        &mut self,
        containing_block_info: &LayoutInfo,
        containing_block_style: Option<&ComputedValues>,
        prev_sibling_pos: Option<BoxPosition>,
        prev_sibling_size: Option<BoxSize>,
    ) {
        assert!(self.child_nodes.is_empty());
        let style = containing_block_style.unwrap();
        let margin = match &self.box_kind {
            BoxKind::Text(_) => style.margin.as_ref().unwrap().clone(),
            _ => unreachable!(),
        };

        // Set used values for the margin, padding, and border properties.
        self.layout_info.used_values.margin.left = match &margin.left {
            CssValue::Ident(v) if v == "auto" => 0.0,
            CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)) => *size,
            CssValue::Percentage(_) => unimplemented!(),
            _ => unreachable!(),
        };
        self.layout_info.used_values.margin.right = match &margin.right {
            CssValue::Ident(v) if v == "auto" => 0.0,
            CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)) => *size,
            CssValue::Percentage(_) => unimplemented!(),
            _ => unreachable!(),
        };
        self.layout_info.used_values.margin.top = match &margin.right {
            CssValue::Ident(v) if v == "auto" => 0.0,
            CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)) => *size,
            CssValue::Percentage(_) => unimplemented!(),
            _ => unreachable!(),
        };
        self.layout_info.used_values.margin.bottom = match &margin.right {
            CssValue::Ident(v) if v == "auto" => 0.0,
            CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)) => *size,
            CssValue::Percentage(_) => unimplemented!(),
            _ => unreachable!(),
        };
        self.layout_info.used_values.width = None;

        // Calculate the width and height of the text box.
        let font_size = if let BoxKind::Text(_) = &self.box_kind {
            let CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)) =
                style.font_size.as_ref().unwrap().size
            else {
                unreachable!()
            };
            size
        } else {
            unreachable!()
        };
        let font = SystemSource::new()
            .select_best_match(&[FamilyName::SansSerif], &Properties::new())
            .unwrap()
            .load()
            .unwrap();
        let text = if let BoxKind::Text(node) = &self.box_kind {
            node.borrow().node.borrow().get_inside_text().unwrap()
        } else {
            unreachable!()
        };
        let metrics: Metrics = font.metrics();
        let scale_factor = font_size / metrics.units_per_em as f32;

        let mut total_width = 0.0;
        for c in text.chars() {
            let glyph_id = font
                .glyph_for_char(c)
                .unwrap_or(font.glyph_for_char('?').unwrap());
            let advance = font.advance(glyph_id);
            total_width += advance.unwrap().x() * scale_factor;
        }
        let max_height = (metrics.ascent - metrics.descent) * scale_factor;

        // Set the width, height, and position of the text box.
        self.layout_info.size.width = total_width;
        self.layout_info.size.height = max_height;
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

    fn layout_children(&mut self) -> Result<()> {
        if self.child_nodes.is_empty() {
            return Ok(());
        }

        let self_style = match &self.box_kind {
            BoxKind::Block(node) | BoxKind::Inline(node) => Some(node.borrow().style.clone()),
            BoxKind::Anonymous(style) => Some(*style.clone()),
            _ => None,
        };

        let (is_first_child_block, is_first_child_anon, is_first_child_inline, is_first_child_text) =
            match self.child_nodes.first().unwrap().borrow().box_kind {
                BoxKind::Block(_) => (true, false, false, false),
                BoxKind::Anonymous(_) => (false, true, false, false),
                BoxKind::Inline(_) => (false, false, true, false),
                BoxKind::Text(_) => (false, false, false, true),
            };

        if is_first_child_block || is_first_child_anon {
            // If the first child is a block-level box, all children must be block-level boxes.
            let mut prev_sib_pos = None;
            let mut prev_sib_size = None;

            // todo: Implement the collapsing margins:
            // https://www.w3.org/TR/CSS22/box.html#collapsing-margins

            // If `height` is `auto`, the height of the box depends on whether the element
            // has any block-level children and whether it has padding or borders.
            // https://www.w3.org/TR/CSS22/visudet.html#normal-block
            for child in self.child_nodes.iter_mut() {
                child.borrow_mut().layout(
                    &self.layout_info,
                    self_style.as_ref(),
                    prev_sib_pos,
                    prev_sib_size,
                )?;
                let ch_ex_pos = child.borrow().get_expanded_pos();
                let ch_ex_size = child.borrow().get_expanded_size();
                self.layout_info.size.height += ch_ex_size.height;
                prev_sib_pos = Some(ch_ex_pos);
                prev_sib_size = Some(ch_ex_size);
            }

            // If `height` is not `auto`, the height of the box is the value of `height`.
            let height = match &self.box_kind {
                BoxKind::Block(node) => node.borrow().style.height.as_ref().unwrap().clone(),
                BoxKind::Anonymous(style) => style.height.as_ref().unwrap().clone(),
                _ => bail!("Invalid box kind: {:?}", self.box_kind),
            };
            if let CssValue::Length(height, _) = height.size {
                self.layout_info.size.height = height;
            }
        } else if is_first_child_inline || is_first_child_text {
            // If the first child is an inline-level box, all children must be inline-level boxes.

            let mut inline_width = 0.0;
            let mut inline_max_height = 0.0;
            let mut prev_sib_pos = None;
            let mut prev_sib_size = None;

            for child in self.child_nodes.iter_mut() {
                if let BoxKind::Block(_) | BoxKind::Anonymous(_) = child.borrow().box_kind {
                    bail!("A block-level box cannot be a sibling of an inline-level box.");
                }
                child.borrow_mut().layout(
                    &self.layout_info,
                    self_style.as_ref(),
                    prev_sib_pos,
                    prev_sib_size,
                )?;
                let ch_exp_pos = child.borrow().get_expanded_pos();
                let ch_exp_size = child.borrow().get_expanded_size();
                inline_width += ch_exp_size.width;
                if inline_max_height < ch_exp_size.height {
                    inline_max_height = ch_exp_size.height;
                }
                prev_sib_pos = Some(ch_exp_pos);
                prev_sib_size = Some(ch_exp_size);
            }

            // If parent is an inline-level box and children are inline-level boxes,
            // the parent's width is the sum of the children's widths.
            // But if parent is a block-level box and children are inline-level boxes,
            // the parent's width is defined by the parent itself (so this if-block should not be executed).
            if let BoxKind::Inline(_) = &self.box_kind {
                self.layout_info.size.width = inline_width;
            }

            self.layout_info.size.height = inline_max_height;
        } else {
            unreachable!();
        }

        Ok(())
    }

    fn calc_used_values_for_inline(&mut self) -> Result<()> {
        let (margin, display) = match &self.box_kind {
            BoxKind::Inline(node) | BoxKind::Text(node) => (
                node.borrow().style.margin.as_ref().unwrap().clone(),
                node.borrow().style.display.as_ref().unwrap().clone(),
            ),
            _ => unreachable!(),
        };
        if (display.outside, display.inside) != (DisplayOutside::Inline, DisplayInside::Flow) {
            unimplemented!("Currently, only inline-level boxes in normal flow are supported.");
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
        Ok(())
    }

    fn calc_inline_pos(
        &mut self,
        containing_block_info: &LayoutInfo,
        prev_sibling_pos: Option<BoxPosition>,
        prev_sibling_size: Option<BoxSize>,
    ) -> Result<()> {
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

        Ok(())
    }

    fn calc_used_values_for_block(&mut self, containing_block_info: &LayoutInfo) -> Result<()> {
        if let BoxKind::Anonymous(_) = &self.box_kind {
            self.layout_info.used_values.width = Some(containing_block_info.size.width);
            self.layout_info.used_values.margin.top = 0.0;
            self.layout_info.used_values.margin.right = 0.0;
            self.layout_info.used_values.margin.bottom = 0.0;
            self.layout_info.used_values.margin.left = 0.0;
            self.layout_info.size.width = containing_block_info.size.width;
            return Ok(());
        }

        let (width, margin, display) = match &self.box_kind {
            BoxKind::Block(node) => (
                node.borrow().style.width.as_ref().unwrap().clone(),
                node.borrow().style.margin.as_ref().unwrap().clone(),
                node.borrow().style.display.as_ref().unwrap().clone(),
            ),
            BoxKind::Anonymous(style) => (
                style.width.as_ref().unwrap().clone(),
                style.margin.as_ref().unwrap().clone(),
                style.display.as_ref().unwrap().clone(),
            ),
            _ => bail!("Invalid box kind: {:?}", &self.box_kind),
        };
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
                        CssValue::Length(
                            size,
                            LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                        ) => *size,
                        CssValue::Percentage(_) => unimplemented!(),
                        _ => unreachable!(),
                    })
                    .sum::<f32>()
                    + self.layout_info.used_values.padding.left
                    + self.layout_info.used_values.padding.right
                    + self.layout_info.used_values.border.left
                    + self.layout_info.used_values.border.right;

                let leeway = containing_block_info.size.width - sum;

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
                            let CssValue::Length(margin_right_px, _) = margin_right else {
                                unreachable!()
                            };
                            let CssValue::Length(margin_left_px, _) = margin_left else {
                                unreachable!()
                            };
                            let CssValue::Length(width_px, _) = width.size else {
                                unreachable!()
                            };
                            (width_px, margin_left_px, margin_right_px + leeway)
                        }
                        (false, true, true) => {
                            let CssValue::Length(width_px, _) = width.size else {
                                unreachable!()
                            };
                            (width_px, leeway / 2.0, leeway / 2.0)
                        }
                        (true, _, _) => {
                            let margin_left_px = match margin_left {
                                CssValue::Ident(v) if v == "auto" => 0.0,
                                CssValue::Length(
                                    size,
                                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                                ) => size,
                                CssValue::Percentage(_) => unimplemented!(),
                                _ => unreachable!(),
                            };
                            let margin_right_px = match margin_right {
                                CssValue::Ident(v) if v == "auto" => 0.0,
                                CssValue::Length(
                                    size,
                                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                                ) => size,
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
                    CssValue::Length(
                        size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    ) => size,
                    CssValue::Percentage(_) => unimplemented!(),
                    _ => unreachable!(),
                };
                self.layout_info.used_values.margin.bottom = match margin.bottom {
                    CssValue::Ident(v) if v == "auto" => 0.0,
                    CssValue::Length(
                        size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    ) => size,
                    CssValue::Percentage(_) => unimplemented!(),
                    _ => unreachable!(),
                };
                // self.layout_info.size.width = width_px;
            }

            _ => unimplemented!("Currently, only block-level boxes in normal flow are supported."),
        }
        Ok(())
    }

    /// https://www.w3.org/TR/CSS22/visudet.html#normal-block
    fn calc_block_pos(
        &mut self,
        containing_block_info: &LayoutInfo,
        prev_sibling_pos: Option<BoxPosition>,
        prev_sibling_size: Option<BoxSize>,
    ) -> Result<()> {
        if let BoxKind::Anonymous(_) = &self.box_kind {
            self.layout_info.pos.x = containing_block_info.pos.x;
            self.layout_info.pos.y =
                if let (Some(BoxPosition { y, .. }), Some(BoxSize { height, .. })) =
                    (&prev_sibling_pos, &prev_sibling_size)
                {
                    y + height
                } else {
                    containing_block_info.pos.y
                };
            return Ok(());
        }

        // Note that the padding used to calculate the coordinates of a block-level box is not the padding of the current box.
        self.layout_info.pos.x = self.layout_info.used_values.margin.left
            + self.layout_info.used_values.border.left
            + containing_block_info.used_values.padding.left
            + containing_block_info.pos.x;
        self.layout_info.pos.y = self.layout_info.used_values.margin.top
            + self.layout_info.used_values.border.top
            + containing_block_info.used_values.padding.top
            + if let (Some(BoxPosition { y, .. }), Some(BoxSize { height, .. })) =
                (&prev_sibling_pos, &prev_sibling_size)
            {
                y + height
            } else {
                containing_block_info.pos.y
            };

        Ok(())
    }
}

impl fmt::Display for BoxNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut fmt_str = String::new();
        match &self.box_kind {
            BoxKind::Block(node) => {
                fmt_str.push_str(&format!("Box: Block, {}", node.borrow()));
            }
            BoxKind::Inline(node) => {
                fmt_str.push_str(&format!("Box: Inline, {}", node.borrow()));
            }
            BoxKind::Text(node) => {
                fmt_str.push_str(&format!("{}", node.borrow()));
            }
            BoxKind::Anonymous(style) => {
                fmt_str.push_str(&format!("Box: Anonymous, Computed( {} )", style));
            }
        }
        fmt_str.push_str(&format!(
            ", (x, y, w, h): ({}, {}, {}, {})",
            self.layout_info.pos.x,
            self.layout_info.pos.y,
            self.layout_info.size.width,
            self.layout_info.size.height
        ));
        write!(f, "{}", fmt_str)
    }
}
