use std::fmt;
use std::rc::Rc;
use std::{cell::RefCell, collections::HashMap};

use crate::css::cssom::{ComponentValue, Declaration, Rule, StyleSheet};
use crate::css::selector::Selector;
use crate::css::tokenizer::{CssToken, NumericType};
use crate::html::dom::DocumentTree;
use crate::html::dom::{DomNode, NodeType};

#[derive(Clone, Debug)]
pub enum SpecifiedValue {
    String(String),
    Integer(i32),
    Float(f32),
}

type SpecifiedValues = HashMap<String, SpecifiedValue>;

#[derive(Debug)]
pub struct RenderNode {
    node: Rc<RefCell<DomNode>>,
    // todo: This should be the computed values:
    // https://www.w3.org/TR/css-cascade-3/#computed
    style: SpecifiedValues,
    child_nodes: Vec<Rc<RefCell<RenderNode>>>,
}

impl RenderNode {
    /// https://www.w3.org/TR/css-cascade-3/#value-stages
    pub fn new(
        node: Rc<RefCell<DomNode>>,
        style_sheet: &StyleSheet,
        parent_style: Option<SpecifiedValues>,
    ) -> Self {
        let style = if let NodeType::Element(_) = &node.borrow().node_type {
            let declared_values = filter(Rc::clone(&node), style_sheet);
            let cascaded_values = cascade(declared_values);
            default(cascaded_values, parent_style)
        } else {
            HashMap::new()
        };

        Self {
            node: Rc::clone(&node),
            style: style.clone(),
            child_nodes: node
                .borrow()
                .child_nodes
                .iter()
                .map(|child| {
                    Rc::new(RefCell::new(Self::new(
                        Rc::clone(child),
                        style_sheet,
                        Some(style.clone()),
                    )))
                })
                .collect::<Vec<_>>(),
        }
    }
}

impl fmt::Display for RenderNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let NodeType::Element(elm) = &self.node.borrow().node_type {
            write!(f, "{:?}: Specified {:?}", elm, self.style)
        } else {
            write!(f, "{:?}", self.node.borrow().node_type)
        }
    }
}

/// Returns all declared values that match the node.
/// https://www.w3.org/TR/css-cascade-3/#filtering
pub fn filter(
    node: Rc<RefCell<DomNode>>,
    style_sheet: &StyleSheet,
) -> Vec<(Selector, Vec<Declaration>)> {
    let mut declared_values = Vec::new();
    style_sheet.rules.iter().for_each(|rule| {
        let matched = rule.matches(Rc::clone(&node));
        if matched.0 {
            let Rule::QualifiedRule(qualified_rule) = rule;
            for selector in matched.1.unwrap() {
                declared_values.push((selector, qualified_rule.declarations.clone()));
            }
        }
    });

    declared_values
}

/// Returns the declared values sorted by precedence in descending order.
/// https://www.w3.org/TR/css-cascade-3/#cascading
pub fn cascade(declared_values: Vec<(Selector, Vec<Declaration>)>) -> Vec<Vec<Declaration>> {
    // Vec<(index, declaration, specificity)>
    let mut sorted_list = declared_values
        .iter()
        .enumerate()
        .map(|(index, (selector, declarations))| {
            (index, declarations.clone(), selector.calc_specificity())
        })
        .collect::<Vec<_>>();

    // Sort by specificity and then by index.
    // If the specificity is the same, the order of the declarations in the stylesheet is preserved (the last declared style gets precedence).
    // Note: The higher the specificity or index, the higher the priority, so this must be sorted in descending order.
    sorted_list.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.0.cmp(&a.0)));

    sorted_list
        .into_iter()
        .map(|(_, declarations, _)| declarations)
        .collect()
}

/// Returns the table of the name and value pairs for the properties.
/// https://www.w3.org/TR/css-cascade-3/#defaulting
pub fn default(
    declarations: Vec<Vec<Declaration>>,
    parent_style: Option<SpecifiedValues>,
) -> SpecifiedValues {
    let mut style_values = HashMap::new();

    // The higher priority declarations are placed at the beginning of the hash table,
    // and the lower priority declarations with the same name are ignored.
    for declaration in declarations {
        for name_and_value in declaration {
            style_values.entry(name_and_value.name).or_insert_with(|| {
                match name_and_value.value.first().cloned().unwrap() {
                    // todo: More accurate handling of the values
                    ComponentValue::PreservedToken(token) => match token {
                        CssToken::Ident(ident) => SpecifiedValue::String(ident),
                        CssToken::String(string) => SpecifiedValue::String(string),
                        CssToken::Number(number) => match number {
                            NumericType::Integer(int) => SpecifiedValue::Integer(int),
                            NumericType::Number(float) => SpecifiedValue::Float(float),
                        },
                        _ => todo!(),
                    },
                    _ => unimplemented!(),
                }
            });
        }
    }

    // Inherit the parent style
    if let Some(parent_style) = parent_style {
        for (name, value) in parent_style {
            style_values.entry(name).or_insert(value);
        }
    }

    style_values
}

#[derive(Debug)]
pub struct RenderTree {
    root: Rc<RefCell<RenderNode>>,
}

impl RenderTree {
    pub fn new(document_tree: DocumentTree, style_sheet: StyleSheet) -> Self {
        Self {
            root: Rc::new(RefCell::new(RenderNode::new(
                Rc::clone(&document_tree.root),
                &style_sheet,
                None,
            ))),
        }
    }
}

impl fmt::Display for RenderTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn construct_node_view(
            node_tree: &mut String,
            node: &Rc<RefCell<RenderNode>>,
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
