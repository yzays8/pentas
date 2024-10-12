use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use anyhow::{ensure, Context, Result};
use regex::Regex;

use crate::renderer::css::property::display::DisplayOutside;
use crate::renderer::html::dom::{Element, NodeType};
use crate::renderer::style::{RenderNode, RenderTree};

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
        let _n = "html".to_string();
        for child in render_tree.root.borrow().child_nodes.iter() {
            if let NodeType::Element(Element { tag_name: _n, .. }) =
                &child.borrow().node.borrow().node_type
            {
                root = Some(Rc::clone(child));
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
                if let BoxKind::Anonymous = child.borrow().box_kind {
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
    Anonymous,
}

#[derive(Debug)]
pub struct BoxNode {
    pub box_kind: BoxKind,
    pub child_nodes: Vec<Rc<RefCell<Self>>>,
}

impl BoxNode {
    pub fn build(node: Rc<RefCell<RenderNode>>) -> Option<Self> {
        match node.borrow().node.borrow().node_type {
            NodeType::Comment(_) | NodeType::DocumentType(_) => return None,
            NodeType::Text(_) => {
                return Some(Self {
                    box_kind: BoxKind::Text(Rc::clone(&node)),
                    child_nodes: vec![],
                })
            }
            _ => {}
        }

        let mut ret = Self {
            box_kind: match node.borrow().get_display_type() {
                DisplayOutside::Block => BoxKind::Block(Rc::clone(&node)),
                DisplayOutside::Inline => BoxKind::Inline(Rc::clone(&node)),
            },
            child_nodes: Vec::new(),
        };
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
                        ret.child_nodes.push(Rc::new(RefCell::new(child)));
                    }
                }
                DisplayOutside::Inline => {
                    if (node.borrow().get_display_type() == DisplayOutside::Block)
                        && (node.borrow().child_nodes.len() > 1)
                    {
                        // Inline-level contents in block formatting context are wrapped in anonymous boxes.
                        let mut anon_box = BoxNode {
                            box_kind: BoxKind::Anonymous,
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
                        ret.child_nodes.push(Rc::new(RefCell::new(anon_box)));
                    } else {
                        let child = Self::build(Rc::clone(&node.borrow().child_nodes[i]));
                        if let Some(child) = child {
                            ret.child_nodes.push(Rc::new(RefCell::new(child)));
                        }
                    }
                }
            }

            i += 1;
        }

        Some(ret)
    }
}

impl fmt::Display for BoxNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.box_kind {
            BoxKind::Block(node) => write!(f, "Box: Block, {}", node.borrow()),
            BoxKind::Inline(node) => write!(f, "Box: Inline, {}", node.borrow()),
            BoxKind::Text(node) => write!(f, "{}", node.borrow()),
            BoxKind::Anonymous => write!(f, "Box: Anonymous"),
        }
    }
}
