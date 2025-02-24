mod block;
mod inline;
mod text;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use anyhow::{Context, Ok, Result, ensure};
use gtk4::pango;

use crate::renderer::html::dom::NodeType;
use crate::renderer::object::RenderObject;
use crate::renderer::style::property::DisplayOutside;
use crate::renderer::style::{RenderNode, RenderTree};
use crate::utils::PrintableTree;
use block::{AnonymousBox, BlockBox};
use inline::InlineBox;
use text::Text;

/// https://www.w3.org/TR/css-display-3/#box-tree
#[derive(Debug)]
pub struct BoxTree {
    pub root: Rc<RefCell<BoxNode>>,
}

impl BoxTree {
    pub fn build(render_tree: &RenderTree, draw_ctx: &pango::Context) -> Result<Self> {
        ensure!(
            render_tree.root.borrow().dom_node.borrow().node_type == NodeType::Document,
            "The root node of the render tree must be a document node."
        );

        // https://www.w3.org/TR/css-display-3/#root-element
        let root = render_tree
            .root
            .borrow()
            .children
            .iter()
            .find(|child| child.borrow().get_tag_name() == Some("html".to_string()))
            .map(Rc::clone);
        ensure!(
            root.is_some(),
            "The element at the root box of the box tree must be an HTML element node."
        );

        Ok(Self {
            root: Rc::new(RefCell::new(
                BoxNode::build(root.unwrap(), None, draw_ctx)
                    .context("Failed to build box tree")?,
            )),
        })
    }

    pub fn layout(self, viewport_width: i32, viewport_height: i32) -> Result<Self> {
        self.root.borrow_mut().layout(
            // The containing block of the root element is initial containing block,
            // which has the dimensions of the viewport and is positioned at the origin of the canvas.
            // https://www.w3.org/TR/CSS22/visudet.html#containing-block-details
            &LayoutInfo {
                size: BoxSize {
                    // https://www.w3.org/TR/CSS22/visuren.html#viewport
                    width: viewport_width as f32,
                    height: viewport_height as f32,
                },
                pos: BoxPosition { x: 0.0, y: 0.0 },
                used_values: UsedValues::default(),
            },
            None,
            None,
        );
        Ok(self)
    }

    /// Removes unnecessary whitespace from all text nodes in the tree.
    /// https://developer.mozilla.org/en-US/docs/Web/API/Document_Object_Model/Whitespace
    pub fn cleanup(self) -> Result<Self> {
        Ok(self.trim_text()?.remove_empty_anonymous_boxes())
    }

    fn trim_text(mut self) -> Result<Self> {
        fn helper(node: &mut Rc<RefCell<BoxNode>>) -> Result<()> {
            if let BoxNode::Text(_) = *node.borrow() {
                return Ok(());
            }

            let mut remove_list: Vec<usize> = vec![];
            let mut n = node.borrow_mut();
            let children_enum = match &mut *n {
                BoxNode::BlockBox(BlockBox { children, .. })
                | BoxNode::InlineBox(InlineBox { children, .. })
                | BoxNode::AnonymousBox(AnonymousBox { children, .. }) => {
                    children.iter_mut().enumerate()
                }
                BoxNode::Text(_) => unreachable!(),
            };
            let children_num = children_enum.len();

            for (i, child) in children_enum {
                if let BoxNode::Text(t) = &mut *child.borrow_mut() {
                    t.trim_text(i == 0, i == children_num - 1)?;
                    if t.style_node
                        .borrow()
                        .dom_node
                        .borrow()
                        .get_inside_text()
                        .unwrap()
                        .is_empty()
                    {
                        remove_list.push(i);
                    }
                }
                if !remove_list.contains(&i) {
                    helper(child)?;
                }
            }
            for i in remove_list.iter().rev() {
                match &mut *n {
                    BoxNode::BlockBox(BlockBox { children, .. })
                    | BoxNode::InlineBox(InlineBox { children, .. })
                    | BoxNode::AnonymousBox(AnonymousBox { children, .. }) => {
                        children.remove(*i);
                    }
                    BoxNode::Text(_) => {}
                }
            }

            Ok(())
        }

        helper(&mut self.root)?;
        Ok(self)
    }

    fn remove_empty_anonymous_boxes(mut self) -> Self {
        fn helper(node: &mut Rc<RefCell<BoxNode>>) {
            if let BoxNode::Text(_) = *node.borrow() {
                return;
            }

            let mut n = node.borrow_mut();
            let children_enum = match &mut *n {
                BoxNode::BlockBox(BlockBox { children, .. })
                | BoxNode::InlineBox(InlineBox { children, .. })
                | BoxNode::AnonymousBox(AnonymousBox { children, .. }) => {
                    children.iter_mut().enumerate()
                }
                BoxNode::Text(_) => unreachable!(),
            };
            let mut remove_list: Vec<usize> = vec![];

            for (i, child) in children_enum {
                if let BoxNode::AnonymousBox(AnonymousBox { children, .. }) = &*child.borrow() {
                    if children.is_empty() {
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
                    BoxNode::BlockBox(BlockBox { children, .. })
                    | BoxNode::InlineBox(InlineBox { children, .. })
                    | BoxNode::AnonymousBox(AnonymousBox { children, .. }) => {
                        children.remove(*i);
                    }
                    BoxNode::Text(_) => {}
                }
            }
        }

        helper(&mut self.root);
        self
    }

    pub fn to_render_objects(
        &self,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Vec<RenderObject> {
        self.root
            .borrow()
            .to_render_objects(viewport_width, viewport_height)
    }

    /// Returns the maximum (width, height) of the box tree.
    pub fn get_max_size(&self) -> (f32, f32) {
        // Assume that the root element is the <html> element, which contains the entire document.
        if let BoxNode::BlockBox(BlockBox { layout_info, .. }) = &*self.root.borrow() {
            (layout_info.size.width, layout_info.size.height)
        } else {
            unreachable!()
        }
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
                BoxNode::BlockBox(BlockBox { children, .. })
                | BoxNode::InlineBox(InlineBox { children, .. })
                | BoxNode::AnonymousBox(AnonymousBox { children, .. }) => {
                    children.iter_mut().enumerate()
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

impl PrintableTree for BoxTree {}

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

/// Layout box trait for the box nodes in the box tree.
pub trait LayoutBox {
    fn layout(
        &mut self,
        containing_block_info: &LayoutInfo,
        parent_info: Option<LayoutInfo>,
        prev_sibling_info: Option<LayoutInfo>,
    );
    fn layout_children(&mut self, containing_block_info: &LayoutInfo);
    fn to_render_objects(&self, viewport_width: i32, viewport_height: i32) -> Vec<RenderObject>;
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
        style_node: Rc<RefCell<RenderNode>>,
        parent_style_node: Option<Rc<RefCell<RenderNode>>>,
        draw_ctx: &pango::Context,
    ) -> Option<Self> {
        match style_node.borrow().dom_node.borrow().node_type {
            NodeType::Document | NodeType::Comment(_) | NodeType::DocumentType(_) => return None,
            NodeType::Text(_) => {
                if parent_style_node.is_none() {
                    unreachable!()
                }
                return Some(Self::Text(Text {
                    style_node: Rc::clone(&style_node),
                    layout_info: LayoutInfo::default(),
                    draw_ctx: draw_ctx.clone(),
                }));
            }
            _ => {}
        }

        // Create box nodes for the children of the current node.
        let mut children: Vec<Rc<RefCell<BoxNode>>> = Vec::new();
        let mut i = 0;
        while i < style_node.borrow().children.len() {
            match style_node.borrow().children[i].borrow().get_display_type() {
                DisplayOutside::Block => {
                    if style_node.borrow().get_display_type() == DisplayOutside::Inline {
                        // todo: It is tricky to handle block-level boxes within an inline box.
                        // https://www.w3.org/TR/CSS22/visuren.html#anonymous-block-level
                        // https://github.com/w3c/csswg-drafts/issues/1477
                        unimplemented!(
                            "Block-level boxes within an inline box are not yet supported: {}",
                            style_node.borrow().dom_node.borrow().node_type
                        );
                    }

                    let child = Self::build(
                        Rc::clone(&style_node.borrow().children[i]),
                        Some(Rc::clone(&style_node)),
                        draw_ctx,
                    );
                    if let Some(child) = child {
                        children.push(Rc::new(RefCell::new(child)));
                    }
                }
                DisplayOutside::Inline => {
                    // If the number of children is greater than 1, wrap all inline-level contents in an anonymous box.
                    if (style_node.borrow().get_display_type() == DisplayOutside::Block)
                        && (style_node.borrow().children.len() > 1)
                    {
                        let mut anon_box = AnonymousBox {
                            style: Box::new(style_node.borrow().style.clone()),
                            layout_info: LayoutInfo::default(),
                            children: vec![],
                        };

                        // If there are successive inline-level contents, they are wrapped in the same anonymous box.
                        // https://www.w3.org/TR/css-inline-3/#root-inline-box
                        while i < style_node.borrow().children.len()
                            && style_node.borrow().children[i].borrow().get_display_type()
                                == DisplayOutside::Inline
                        {
                            let child = Self::build(
                                Rc::clone(&style_node.borrow().children[i]),
                                Some(Rc::clone(&style_node)),
                                draw_ctx,
                            );
                            if let Some(child) = child {
                                anon_box.children.push(Rc::new(RefCell::new(child)));
                            }
                            i += 1;
                        }
                        i -= 1;
                        children.push(Rc::new(RefCell::new(Self::AnonymousBox(anon_box))));
                    } else {
                        let child = Self::build(
                            Rc::clone(&style_node.borrow().children[i]),
                            Some(Rc::clone(&style_node)),
                            draw_ctx,
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
        let padding = style_node.borrow().style.padding.to_px().unwrap();
        let border = style_node
            .borrow()
            .style
            .border
            .border_width
            .to_px()
            .unwrap();

        match style_node.borrow().get_display_type() {
            DisplayOutside::Block => Some(Self::BlockBox(BlockBox {
                style_node: Rc::clone(&style_node),
                layout_info: LayoutInfo {
                    used_values: UsedValues {
                        padding,
                        border,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                children,
            })),
            DisplayOutside::Inline => Some(Self::InlineBox(InlineBox {
                style_node: Rc::clone(&style_node),
                layout_info: LayoutInfo {
                    used_values: UsedValues {
                        padding,
                        border,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                children,
            })),
        }
    }

    /// Sets the width, height, position, and used values for some properties of the box and its children.
    pub fn layout(
        &mut self,
        // https://www.w3.org/TR/CSS22/visudet.html#containing-block-details
        containing_block_info: &LayoutInfo,
        parent_info: Option<LayoutInfo>,
        prev_sibling_info: Option<LayoutInfo>,
    ) -> &mut Self {
        match self {
            Self::BlockBox(b) => {
                b.layout(containing_block_info, parent_info, prev_sibling_info);
            }
            Self::AnonymousBox(b) => {
                b.layout(containing_block_info, parent_info, prev_sibling_info);
            }
            Self::InlineBox(b) => {
                b.layout(containing_block_info, parent_info, prev_sibling_info);
            }
            Self::Text(t) => {
                t.layout(containing_block_info, parent_info, prev_sibling_info);
            }
        }
        self
    }

    pub fn to_render_objects(
        &self,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Vec<RenderObject> {
        let mut objects = Vec::new();

        match self {
            BoxNode::Text(t) => {
                objects.extend(t.to_render_objects(viewport_width, viewport_height));
            }
            BoxNode::BlockBox(block) => {
                objects.extend(block.to_render_objects(viewport_width, viewport_height));
            }
            BoxNode::InlineBox(inline) => {
                objects.extend(inline.to_render_objects(viewport_width, viewport_height));
            }
            BoxNode::AnonymousBox(anonymous) => {
                objects.extend(anonymous.to_render_objects(viewport_width, viewport_height));
            }
        }

        objects
    }
}

impl fmt::Display for BoxNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut fmt_str = String::new();
        match self {
            Self::BlockBox(BlockBox {
                style_node: node, ..
            }) => {
                fmt_str.push_str(&format!("Box: Block, {}", node.borrow()));
            }
            Self::InlineBox(InlineBox {
                style_node: node, ..
            }) => {
                fmt_str.push_str(&format!("Box: Inline, {}", node.borrow()));
            }
            Self::Text(Text {
                style_node: node, ..
            }) => {
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
