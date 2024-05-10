use core::fmt;
use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{bail, Result};

/// https://dom.spec.whatwg.org/#node
#[derive(Debug, PartialEq, Eq)]
pub struct DomNode {
    pub node_type: NodeType,
    pub child_nodes: Vec<Rc<RefCell<DomNode>>>,
}

impl DomNode {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            child_nodes: Vec::new(),
        }
    }
}

impl fmt::Display for DomNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let NodeType::Element(elm) = &self.node_type {
            write!(f, "{:?}", elm)
        } else {
            write!(f, "{:?}", self.node_type)
        }
    }
}

/// https://dom.spec.whatwg.org/#dom-node-nodetype
#[derive(Debug, PartialEq, Eq)]
pub enum NodeType {
    Document,
    DocumentType(String),
    Element(Element),
    Text(String),
}

/// https://dom.spec.whatwg.org/#element
#[derive(Debug, PartialEq, Eq)]
pub struct Element {
    pub tag_name: String,
    pub attributes: Vec<(String, String)>,
}

/// https://dom.spec.whatwg.org/#document-trees
#[derive(Debug)]
pub struct DocumentTree {
    pub root: Rc<RefCell<DomNode>>,
}

impl DocumentTree {
    pub fn build(root: Rc<RefCell<DomNode>>) -> Result<Self> {
        if root.borrow().node_type != NodeType::Document {
            bail!("The root node of a document tree must be a document node.");
        }
        Ok(Self { root })
    }

    #[allow(dead_code)]
    pub fn get_dfs_iter(&self) -> impl Iterator<Item = Rc<RefCell<DomNode>>> {
        let mut stack = vec![Rc::clone(&self.root)];
        std::iter::from_fn(move || -> Option<Rc<RefCell<DomNode>>> {
            let current = stack.pop()?;
            stack.extend(current.borrow().child_nodes.iter().map(Rc::clone).rev());
            Some(Rc::clone(&current))
        })
    }
}

impl fmt::Display for DocumentTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn print_node(
            node_tree: &mut String,
            node: &Rc<RefCell<DomNode>>,
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
                print_node(
                    node_tree,
                    child,
                    current_depth + 1,
                    i == children_num - 1,
                    exclude_branches.clone(),
                );
            }
        }
        let mut node_tree = String::new();
        print_node(&mut node_tree, &self.root, 0, true, vec![]);
        node_tree.pop(); // Remove the last newline character
        write!(f, "{}", node_tree)
    }
}
