use std::{
    cell::RefCell,
    fmt,
    rc::{Rc, Weak},
};

use crate::{
    error::{Error, Result},
    renderer::{css::cssom::StyleSheet, style::RenderTree},
    utils::PrintableTree,
};

/// https://dom.spec.whatwg.org/#node
#[derive(Debug)]
pub struct DomNode {
    pub node_type: NodeType,
    pub children: Vec<Rc<RefCell<Self>>>,
    pub parent: Option<Weak<RefCell<Self>>>,
    pub prev_sibling: Option<Weak<RefCell<Self>>>,
    pub next_sibling: Option<Rc<RefCell<Self>>>,
}

impl Default for DomNode {
    fn default() -> Self {
        Self {
            node_type: NodeType::Document,
            children: Vec::new(),
            parent: None,
            prev_sibling: None,
            next_sibling: None,
        }
    }
}

impl DomNode {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            ..Default::default()
        }
    }

    pub fn append_child(node: &Rc<RefCell<Self>>, child: Self) -> Rc<RefCell<Self>> {
        let child = Rc::new(RefCell::new(child));
        child.borrow_mut().parent = Some(Rc::downgrade(node));
        if node.borrow().children.is_empty() {
            child.borrow_mut().prev_sibling = None;
            child.borrow_mut().next_sibling = None;
        } else {
            let last_child = Rc::clone(node.borrow().children.last().unwrap());
            child.borrow_mut().prev_sibling = Some(Rc::downgrade(&last_child));
            last_child.borrow_mut().next_sibling = Some(Rc::clone(&child));
        }
        node.borrow_mut().children.push(Rc::clone(&child));
        child
    }

    pub fn get_inside_text(&self) -> Option<String> {
        match &self.node_type {
            NodeType::Comment(text) | NodeType::DocumentType(text) | NodeType::Text(text) => {
                Some(text.clone())
            }
            _ => None,
        }
    }

    pub fn set_inside_text(&mut self, text: &str) {
        if let NodeType::Text(t) = &mut self.node_type {
            *t = text.to_string();
        }
    }

    pub fn get_tag_name(&self) -> Option<String> {
        if let NodeType::Element(elm) = &self.node_type {
            Some(elm.tag_name.clone())
        } else {
            None
        }
    }
}

impl fmt::Display for DomNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let NodeType::Element(elm) = &self.node_type {
            write!(f, "{}", elm)
        } else {
            write!(f, "{:?}", self.node_type)
        }
    }
}

/// https://dom.spec.whatwg.org/#dom-node-nodetype
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Comment(String),
    Document,
    DocumentType(String),
    Element(Element),
    Text(String),
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeType::Comment(text) => write!(f, "Comment( {} )", text),
            NodeType::Document => write!(f, "Document"),
            NodeType::DocumentType(text) => write!(f, "DocumentType( {} )", text),
            NodeType::Element(elm) => write!(f, "{}", elm),
            NodeType::Text(text) => write!(f, "Text( {} )", text),
        }
    }
}

/// https://dom.spec.whatwg.org/#element
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub tag_name: String,
    pub attributes: Vec<(String, String)>,
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let attr = self
            .attributes
            .iter()
            .map(|(key, value)| format!("\"{}\"=\"{}\"", key, value))
            .collect::<Vec<String>>();
        if attr.is_empty() {
            write!(f, "Elem( tag: <{}> )", self.tag_name)
        } else {
            write!(
                f,
                "Elem( tag: <{}>, attr: [{}] )",
                self.tag_name,
                attr.join("; ")
            )
        }
    }
}

/// https://dom.spec.whatwg.org/#document-trees
#[derive(Debug)]
pub struct DocumentTree {
    pub root: Rc<RefCell<DomNode>>,
}

impl DocumentTree {
    pub fn build(root: Rc<RefCell<DomNode>>) -> Result<Self> {
        if root.borrow().node_type != NodeType::Document {
            return Err(Error::Other(
                "The root node of a document tree must be a document node.".into(),
            ));
        }
        Ok(Self { root })
    }

    pub fn to_render_tree(
        &self,
        style_sheets: Vec<StyleSheet>,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<RenderTree> {
        RenderTree::build(self, style_sheets, viewport_width, viewport_height)
    }

    #[cfg(test)]
    pub fn get_dfs_iter(&self) -> impl Iterator<Item = Rc<RefCell<DomNode>>> {
        let mut stack = vec![Rc::clone(&self.root)];
        std::iter::from_fn(move || -> Option<Rc<RefCell<DomNode>>> {
            let current = stack.pop()?;
            stack.extend(current.borrow().children.iter().map(Rc::clone).rev());
            Some(Rc::clone(&current))
        })
    }
}

impl fmt::Display for DocumentTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn construct_node_view(
            node_tree: &mut String,
            node: &DomNode,
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
            node_tree.push_str(&format!("{}{}\n", indent_and_branches, node));
            let children_num = node.children.len();
            for (i, child) in node.children.iter().enumerate() {
                construct_node_view(
                    node_tree,
                    &child.borrow(),
                    current_depth + 1,
                    i == children_num - 1,
                    exclude_branches.clone(),
                );
            }
        }
        let mut node_tree = String::new();
        construct_node_view(&mut node_tree, &self.root.borrow(), 0, true, vec![]);
        node_tree.pop(); // Remove the last newline character
        write!(f, "{}", node_tree)
    }
}

impl PrintableTree for DocumentTree {}
