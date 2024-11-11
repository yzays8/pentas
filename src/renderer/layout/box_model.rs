use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use anyhow::{ensure, Context, Ok, Result};
use regex::Regex;

use crate::renderer::html::dom::{Element, NodeType};
use crate::renderer::layout::block::{AnonymousBox, BlockBox};
use crate::renderer::layout::inline::InlineBox;
use crate::renderer::layout::text::Text;
use crate::renderer::style::property::display::DisplayOutside;
use crate::renderer::style::render_tree::{RenderNode, RenderTree};
use crate::renderer::style::value_type::CssValue;

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
                BoxNode::build(root.unwrap(), None).context("Failed to build box tree")?,
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
        );
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
            if let BoxNode::Text(Text { node, .. }) = &mut *node.borrow_mut() {
                let text = node.borrow().node.borrow().get_inside_text().unwrap();

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

                let text = match node.borrow().get_display_type() {
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

                node.borrow_mut().node.borrow_mut().node_type = NodeType::Text(text.to_string());
            }

            Ok(())
        }

        fn traverse_and_remove_whitespace(node: &mut Rc<RefCell<BoxNode>>) -> Result<()> {
            if let BoxNode::Text(_) = *node.borrow() {
                return Ok(());
            }

            let mut remove_list: Vec<usize> = vec![];
            let mut n = node.borrow_mut();
            let children_enum = match &mut *n {
                BoxNode::BlockBox(BlockBox { child_nodes, .. })
                | BoxNode::InlineBox(InlineBox { child_nodes, .. })
                | BoxNode::AnonymousBox(AnonymousBox { child_nodes, .. }) => {
                    child_nodes.iter_mut().enumerate()
                }
                BoxNode::Text(_) => unreachable!(),
            };
            let children_num = children_enum.len();

            for (i, child) in children_enum {
                helper(child, i == 0, i == children_num - 1)?;
                if let BoxNode::Text(Text { node, .. }) = &*child.borrow() {
                    if node
                        .borrow()
                        .node
                        .borrow()
                        .get_inside_text()
                        .unwrap()
                        .is_empty()
                    {
                        remove_list.push(i);
                    }
                }
                if !remove_list.contains(&i) {
                    traverse_and_remove_whitespace(child)?;
                }
            }
            for i in remove_list.iter().rev() {
                match &mut *n {
                    BoxNode::BlockBox(BlockBox { child_nodes, .. })
                    | BoxNode::InlineBox(InlineBox { child_nodes, .. })
                    | BoxNode::AnonymousBox(AnonymousBox { child_nodes, .. }) => {
                        child_nodes.remove(*i);
                    }
                    BoxNode::Text(_) => {}
                }
            }

            Ok(())
        }

        traverse_and_remove_whitespace(&mut self.root)?;
        Ok(self)
    }

    fn remove_empty_anonymous_boxes(&mut self) -> &mut Self {
        fn helper(node: &mut Rc<RefCell<BoxNode>>) {
            if let BoxNode::Text(_) = *node.borrow() {
                return;
            }

            let mut n = node.borrow_mut();
            let children_enum = match &mut *n {
                BoxNode::BlockBox(BlockBox { child_nodes, .. })
                | BoxNode::InlineBox(InlineBox { child_nodes, .. })
                | BoxNode::AnonymousBox(AnonymousBox { child_nodes, .. }) => {
                    child_nodes.iter_mut().enumerate()
                }
                BoxNode::Text(_) => unreachable!(),
            };
            let mut remove_list: Vec<usize> = vec![];

            for (i, child) in children_enum {
                if let BoxNode::AnonymousBox(AnonymousBox { child_nodes, .. }) = &*child.borrow() {
                    if child_nodes.is_empty() {
                        remove_list.push(i);
                    }
                }

                // If the child is not removed as an empty anonymous box, recursively check its children.
                if !remove_list.contains(&i) {
                    helper(child);
                }
            }
            for i in remove_list.iter().rev() {
                match &mut *n {
                    BoxNode::BlockBox(BlockBox { child_nodes, .. })
                    | BoxNode::InlineBox(InlineBox { child_nodes, .. })
                    | BoxNode::AnonymousBox(AnonymousBox { child_nodes, .. }) => {
                        child_nodes.remove(*i);
                    }
                    BoxNode::Text(_) => {}
                }
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

            // No children.
            if let BoxNode::Text(_) = *node.borrow() {
                return;
            }

            let mut n = node.borrow_mut();
            let children_enum = match &mut *n {
                BoxNode::BlockBox(BlockBox { child_nodes, .. })
                | BoxNode::InlineBox(InlineBox { child_nodes, .. })
                | BoxNode::AnonymousBox(AnonymousBox { child_nodes, .. }) => {
                    child_nodes.iter_mut().enumerate()
                }
                BoxNode::Text(_) => unreachable!(),
            };
            let children_num = children_enum.len();
            for (i, child) in children_enum {
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

/// Calculated width, height, and position of the BoxNode and its `used values` of the `width`, `margin`, `padding`, and `border` properties.
#[derive(Debug, Default, Clone)]
pub struct LayoutInfo {
    pub size: BoxSize,
    pub pos: BoxPosition,
    pub used_values: UsedValues,
}

impl LayoutInfo {
    /// Returns the width of the box after applying the margin, padding, and border properties.
    pub fn get_expanded_size(&self) -> BoxSize {
        BoxSize {
            width: self.size.width
                + self.used_values.margin.left
                + self.used_values.margin.right
                + self.used_values.padding.left
                + self.used_values.padding.right
                + self.used_values.border.left
                + self.used_values.border.right,
            height: self.size.height
                + self.used_values.margin.top
                + self.used_values.margin.bottom
                + self.used_values.padding.top
                + self.used_values.padding.bottom
                + self.used_values.border.top
                + self.used_values.border.bottom,
        }
    }

    /// Returns the position of the box after applying the margin, padding, and border properties.
    pub fn get_expanded_pos(&self) -> BoxPosition {
        BoxPosition {
            x: self.pos.x
                - self.used_values.margin.left
                - self.used_values.padding.left
                - self.used_values.border.left,
            y: self.pos.y
                - self.used_values.margin.top
                - self.used_values.padding.top
                - self.used_values.border.top,
        }
    }
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
#[derive(Debug, Default, Clone)]
pub struct UsedValues {
    /// The `width` property won't be applied to non-replaced inline elements.
    pub width: Option<f32>,
    pub margin: Edge,
    pub padding: Edge,
    pub border: Edge,
}

#[derive(Debug, Default, Clone)]
pub struct Edge {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug)]
pub enum BoxNode {
    /// https://www.w3.org/TR/css-display-3/#block-box
    BlockBox(BlockBox),

    /// https://www.w3.org/TR/css-display-3/#inline-box
    InlineBox(InlineBox),

    /// https://www.w3.org/TR/css-display-3/#css-text-sequence
    Text(Text),

    /// https://www.w3.org/TR/css-display-3/#anonymous
    AnonymousBox(AnonymousBox),
}

impl BoxNode {
    pub fn build(
        node: Rc<RefCell<RenderNode>>,
        parent: Option<Rc<RefCell<RenderNode>>>,
    ) -> Option<Self> {
        match node.borrow().node.borrow().node_type {
            NodeType::Document | NodeType::Comment(_) | NodeType::DocumentType(_) => return None,
            NodeType::Text(_) => {
                if parent.is_none() {
                    unreachable!()
                }
                return Some(Self::Text(Text {
                    node: Rc::clone(&node),
                    layout_info: LayoutInfo::default(),
                    parent: Rc::downgrade(&parent.unwrap()),
                }));
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

                    let child = Self::build(
                        Rc::clone(&node.borrow().child_nodes[i]),
                        Some(Rc::clone(&node)),
                    );
                    if let Some(child) = child {
                        children.push(Rc::new(RefCell::new(child)));
                    }
                }
                DisplayOutside::Inline => {
                    // If the number of children is greater than 1, wrap all inline-level contents in an anonymous box.
                    if (node.borrow().get_display_type() == DisplayOutside::Block)
                        && (node.borrow().child_nodes.len() > 1)
                    {
                        let mut anon_box = AnonymousBox {
                            style: Box::new(node.borrow().style.clone()),
                            layout_info: LayoutInfo::default(),
                            child_nodes: vec![],
                        };

                        // If there are successive inline-level contents, they are wrapped in the same anonymous box.
                        // https://www.w3.org/TR/css-inline-3/#root-inline-box
                        while i < node.borrow().child_nodes.len()
                            && node.borrow().child_nodes[i].borrow().get_display_type()
                                == DisplayOutside::Inline
                        {
                            let child = Self::build(
                                Rc::clone(&node.borrow().child_nodes[i]),
                                Some(Rc::clone(&node)),
                            );
                            if let Some(child) = child {
                                anon_box.child_nodes.push(Rc::new(RefCell::new(child)));
                            }
                            i += 1;
                        }
                        i -= 1;
                        children.push(Rc::new(RefCell::new(Self::AnonymousBox(anon_box))));
                    } else {
                        let child = Self::build(
                            Rc::clone(&node.borrow().child_nodes[i]),
                            Some(Rc::clone(&node)),
                        );
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

        match node.borrow().get_display_type() {
            DisplayOutside::Block => Some(Self::BlockBox(BlockBox {
                node: Rc::clone(&node),
                layout_info: LayoutInfo {
                    used_values: UsedValues {
                        padding,
                        border,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                child_nodes: children,
            })),
            DisplayOutside::Inline => Some(Self::InlineBox(InlineBox {
                node: Rc::clone(&node),
                layout_info: LayoutInfo {
                    used_values: UsedValues {
                        padding,
                        border,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                child_nodes: children,
            })),
        }
    }

    /// Sets the width, height, position, and used values for some properties of the box and its children.
    pub fn layout(
        &mut self,
        containing_block_info: &LayoutInfo,
        prev_sibling_info: Option<LayoutInfo>,
    ) -> &mut Self {
        match self {
            Self::BlockBox(b) => {
                b.layout(containing_block_info, prev_sibling_info);
            }
            Self::AnonymousBox(b) => {
                b.layout(containing_block_info, prev_sibling_info);
            }
            Self::InlineBox(b) => {
                b.layout(containing_block_info, prev_sibling_info);
            }
            Self::Text(t) => {
                t.layout(containing_block_info, prev_sibling_info);
            }
        }
        self
    }
}

impl fmt::Display for BoxNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut fmt_str = String::new();
        match self {
            Self::BlockBox(BlockBox { node, .. }) => {
                fmt_str.push_str(&format!("Box: Block, {}", node.borrow()));
            }
            Self::InlineBox(InlineBox { node, .. }) => {
                fmt_str.push_str(&format!("Box: Inline, {}", node.borrow()));
            }
            Self::Text(Text { node, .. }) => {
                fmt_str.push_str(&format!("{}", node.borrow()));
            }
            Self::AnonymousBox(AnonymousBox { style, .. }) => {
                fmt_str.push_str(&format!("Box: Anonymous, Computed( {} )", style));
            }
        }
        let layout_info = match self {
            Self::BlockBox(BlockBox { layout_info, .. })
            | Self::InlineBox(InlineBox { layout_info, .. })
            | Self::Text(Text { layout_info, .. })
            | Self::AnonymousBox(AnonymousBox { layout_info, .. }) => layout_info,
        };
        fmt_str.push_str(&format!(
            ", (x, y, w, h): ({}, {}, {}, {})",
            layout_info.pos.x, layout_info.pos.y, layout_info.size.width, layout_info.size.height
        ));
        write!(f, "{}", fmt_str)
    }
}
