use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use anyhow::{ensure, Context, Ok, Result};
use gtk4::pango;

use crate::renderer::html::dom::{Element, NodeType};
use crate::renderer::layout::block::{AnonymousBox, BlockBox};
use crate::renderer::layout::inline::InlineBox;
use crate::renderer::layout::text::Text;
use crate::renderer::style::property::display::DisplayOutside;
use crate::renderer::style::property::CssValue;
use crate::renderer::style::style_model::{RenderNode, RenderTree};
use crate::renderer::utils::PrintableTree;
use crate::renderer::RenderObject;

/// https://www.w3.org/TR/css-display-3/#box-tree
#[derive(Debug)]
pub struct BoxTree {
    pub root: Rc<RefCell<BoxNode>>,
}

impl BoxTree {
    pub fn build(render_tree: &RenderTree, draw_ctx: &pango::Context) -> Result<Self> {
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
                BoxNode::build(root.unwrap(), None, draw_ctx)
                    .context("Failed to build box tree")?,
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
        );
        Ok(self)
    }

    /// Removes unnecessary whitespace from all text nodes in the tree.
    /// https://developer.mozilla.org/en-US/docs/Web/API/Document_Object_Model/Whitespace
    pub fn clean_up(&mut self) -> Result<&mut Self> {
        Ok(self.trim_text()?.remove_empty_anonymous_boxes())
    }

    fn trim_text(&mut self) -> Result<&mut Self> {
        fn helper(node: &mut Rc<RefCell<BoxNode>>) -> Result<()> {
            if let BoxNode::Text(_) = *node.borrow() {
                return Ok(());
            }

            let mut remove_list: Vec<usize> = vec![];
            let mut n = node.borrow_mut();
            let children_enum = match &mut *n {
                BoxNode::BlockBox(BlockBox {
                    children: child_nodes,
                    ..
                })
                | BoxNode::InlineBox(InlineBox {
                    children: child_nodes,
                    ..
                })
                | BoxNode::AnonymousBox(AnonymousBox {
                    children: child_nodes,
                    ..
                }) => child_nodes.iter_mut().enumerate(),
                BoxNode::Text(_) => unreachable!(),
            };
            let children_num = children_enum.len();

            for (i, child) in children_enum {
                if let BoxNode::Text(t) = &mut *child.borrow_mut() {
                    t.trim_text(i == 0, i == children_num - 1)?;
                    if t.style_node
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
                    helper(child)?;
                }
            }
            for i in remove_list.iter().rev() {
                match &mut *n {
                    BoxNode::BlockBox(BlockBox {
                        children: child_nodes,
                        ..
                    })
                    | BoxNode::InlineBox(InlineBox {
                        children: child_nodes,
                        ..
                    })
                    | BoxNode::AnonymousBox(AnonymousBox {
                        children: child_nodes,
                        ..
                    }) => {
                        child_nodes.remove(*i);
                    }
                    BoxNode::Text(_) => {}
                }
            }

            Ok(())
        }

        helper(&mut self.root)?;
        Ok(self)
    }

    fn remove_empty_anonymous_boxes(&mut self) -> &mut Self {
        fn helper(node: &mut Rc<RefCell<BoxNode>>) {
            if let BoxNode::Text(_) = *node.borrow() {
                return;
            }

            let mut n = node.borrow_mut();
            let children_enum = match &mut *n {
                BoxNode::BlockBox(BlockBox {
                    children: child_nodes,
                    ..
                })
                | BoxNode::InlineBox(InlineBox {
                    children: child_nodes,
                    ..
                })
                | BoxNode::AnonymousBox(AnonymousBox {
                    children: child_nodes,
                    ..
                }) => child_nodes.iter_mut().enumerate(),
                BoxNode::Text(_) => unreachable!(),
            };
            let mut remove_list: Vec<usize> = vec![];

            for (i, child) in children_enum {
                if let BoxNode::AnonymousBox(AnonymousBox {
                    children: child_nodes,
                    ..
                }) = &*child.borrow()
                {
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
                    BoxNode::BlockBox(BlockBox {
                        children: child_nodes,
                        ..
                    })
                    | BoxNode::InlineBox(InlineBox {
                        children: child_nodes,
                        ..
                    })
                    | BoxNode::AnonymousBox(AnonymousBox {
                        children: child_nodes,
                        ..
                    }) => {
                        child_nodes.remove(*i);
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
        let mut objects = Vec::new();
        self.root
            .borrow()
            .to_render_objects(&mut objects, viewport_width, viewport_height);
        objects
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
                BoxNode::BlockBox(BlockBox {
                    children: child_nodes,
                    ..
                })
                | BoxNode::InlineBox(InlineBox {
                    children: child_nodes,
                    ..
                })
                | BoxNode::AnonymousBox(AnonymousBox {
                    children: child_nodes,
                    ..
                }) => child_nodes.iter_mut().enumerate(),
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
        match style_node.borrow().node.borrow().node_type {
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
        while i < style_node.borrow().child_nodes.len() {
            match style_node.borrow().child_nodes[i]
                .borrow()
                .get_display_type()
            {
                DisplayOutside::Block => {
                    if style_node.borrow().get_display_type() == DisplayOutside::Inline {
                        // todo: It is tricky to handle block-level boxes within an inline box.
                        // https://www.w3.org/TR/CSS22/visuren.html#anonymous-block-level
                        // https://github.com/w3c/csswg-drafts/issues/1477
                        unimplemented!(
                            "Block-level boxes within an inline box are not yet supported: {}",
                            style_node.borrow().node.borrow().node_type
                        );
                    }

                    let child = Self::build(
                        Rc::clone(&style_node.borrow().child_nodes[i]),
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
                        && (style_node.borrow().child_nodes.len() > 1)
                    {
                        let mut anon_box = AnonymousBox {
                            style_node: Box::new(style_node.borrow().style.clone()),
                            layout_info: LayoutInfo::default(),
                            children: vec![],
                        };

                        // If there are successive inline-level contents, they are wrapped in the same anonymous box.
                        // https://www.w3.org/TR/css-inline-3/#root-inline-box
                        while i < style_node.borrow().child_nodes.len()
                            && style_node.borrow().child_nodes[i]
                                .borrow()
                                .get_display_type()
                                == DisplayOutside::Inline
                        {
                            let child = Self::build(
                                Rc::clone(&style_node.borrow().child_nodes[i]),
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
                            Rc::clone(&style_node.borrow().child_nodes[i]),
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
        let padding = if let (
            CssValue::Length(top_px, _),
            CssValue::Length(right_px, _),
            CssValue::Length(bottom_px, _),
            CssValue::Length(left_px, _),
        ) = (
            &style_node.borrow().style.padding.top,
            &style_node.borrow().style.padding.right,
            &style_node.borrow().style.padding.bottom,
            &style_node.borrow().style.padding.left,
        ) {
            Edge {
                top: *top_px,
                right: *right_px,
                bottom: *bottom_px,
                left: *left_px,
            }
        } else {
            unreachable!()
        };
        let border = if let (
            CssValue::Length(top_px, _),
            CssValue::Length(right_px, _),
            CssValue::Length(bottom_px, _),
            CssValue::Length(left_px, _),
        ) = (
            &style_node.borrow().style.border.border_width.top,
            &style_node.borrow().style.border.border_width.right,
            &style_node.borrow().style.border.border_width.bottom,
            &style_node.borrow().style.border.border_width.left,
        ) {
            Edge {
                top: *top_px,
                right: *right_px,
                bottom: *bottom_px,
                left: *left_px,
            }
        } else {
            unreachable!();
        };

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
        objects: &mut Vec<RenderObject>,
        viewport_width: i32,
        viewport_height: i32,
    ) {
        match self {
            BoxNode::Text(t) => {
                let CssValue::Length(font_size_px, _) = t.style_node.borrow().style.font_size.size
                else {
                    unreachable!()
                };
                let font_family = t.style_node.borrow().style.font_family.to_name_list();
                let font_weight = &t.style_node.borrow().style.font_weight.to_name();
                let color = if let CssValue::Color { r, g, b, a } =
                    t.style_node.borrow().style.color.value
                {
                    (r, g, b, a)
                } else {
                    return;
                };
                let decoration_color = if let CssValue::Color { r, g, b, a } =
                    t.style_node.borrow().style.text_decoration.color.value
                {
                    (r, g, b, a)
                } else {
                    unreachable!()
                };
                let decoration_line = t
                    .style_node
                    .borrow()
                    .style
                    .text_decoration
                    .line
                    .iter()
                    .map(|v| {
                        if let CssValue::Ident(v) = v {
                            v.clone()
                        } else {
                            unreachable!()
                        }
                    })
                    .collect::<Vec<String>>();
                let CssValue::Ident(decoration_style) =
                    t.style_node.borrow().style.text_decoration.style.clone()
                else {
                    unreachable!()
                };
                objects.push(RenderObject::Text {
                    text: t
                        .style_node
                        .borrow()
                        .node
                        .borrow()
                        .get_inside_text()
                        .unwrap(),
                    x: t.layout_info.pos.x as f64,
                    y: t.layout_info.pos.y as f64,
                    font_family,
                    font_size: font_size_px as f64,
                    font_weight: font_weight.to_string(),
                    color: (
                        color.0 as f64 / 255.0,
                        color.1 as f64 / 255.0,
                        color.2 as f64 / 255.0,
                    ),
                    decoration_color: (
                        decoration_color.0 as f64 / 255.0,
                        decoration_color.1 as f64 / 255.0,
                        decoration_color.2 as f64 / 255.0,
                    ),
                    decoration_line,
                    decoration_style,
                });
            }
            BoxNode::BlockBox(block) => {
                let (r, g, b, a) = if let CssValue::Color { r, g, b, a } =
                    block.style_node.borrow().style.background_color.value
                {
                    (r, g, b, a)
                } else {
                    for child in block.children.iter() {
                        child
                            .borrow()
                            .to_render_objects(objects, viewport_width, viewport_height);
                    }
                    return;
                };
                // The style of the body element is applied to the whole viewport.
                let is_body = if let NodeType::Element(Element { tag_name: n, .. }) =
                    &block.style_node.borrow().node.borrow().node_type
                {
                    n == "body"
                } else {
                    false
                };

                // transparent
                if a != 0.0 {
                    objects.push(RenderObject::Rectangle {
                        x: block.layout_info.pos.x as f64,
                        y: block.layout_info.pos.y as f64,
                        width: if is_body {
                            viewport_width as f64
                        } else {
                            block.layout_info.size.width as f64
                        },
                        height: if is_body {
                            viewport_height as f64
                        } else {
                            block.layout_info.size.height as f64
                        },
                        color: (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0),
                    });
                }
                for child in block.children.iter() {
                    child
                        .borrow()
                        .to_render_objects(objects, viewport_width, viewport_height);
                }
            }
            BoxNode::InlineBox(inline) => {
                for child in inline.children.iter() {
                    child
                        .borrow()
                        .to_render_objects(objects, viewport_width, viewport_height);
                }
            }
            BoxNode::AnonymousBox(anonymous) => {
                for child in anonymous.children.iter() {
                    child
                        .borrow()
                        .to_render_objects(objects, viewport_width, viewport_height);
                }
            }
        }
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
            Self::AnonymousBox(AnonymousBox {
                style_node: style, ..
            }) => {
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
