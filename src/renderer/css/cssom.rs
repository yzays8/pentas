use std::{cell::RefCell, rc::Rc};

use crate::renderer::{
    css::{selector::Selector, token::CssToken},
    html::dom::DomNode,
};

/// https://www.w3.org/TR/cssom-1/#cssstylesheet
#[derive(Debug, Clone)]
pub struct StyleSheet {
    pub rules: Vec<Rule>,
}

impl StyleSheet {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    pub fn print(&self) {
        println!("{:#?}", self);
    }
}

/// A CSS document is a series of style rules and at-rules.
/// - https://www.w3.org/TR/css-syntax-3/#syntax-description
/// - https://www.w3.org/TR/cssom-1/#cssrule
#[derive(Debug, Clone, PartialEq)]
pub enum Rule {
    QualifiedRule(QualifiedRule),
    AtRule(AtRule),
}

impl Rule {
    pub fn get_matched_selectors(&self, dom_node: Rc<RefCell<DomNode>>) -> Option<Vec<Selector>> {
        match self {
            Rule::QualifiedRule(rule) => rule.get_matched_selectors(dom_node),
            _ => None,
        }
    }
}

/// https://www.w3.org/TR/css-syntax-3/#qualified-rule
pub type QualifiedRule = StyleRule;

/// - https://www.w3.org/TR/css-syntax-3/#style-rules
/// - https://www.w3.org/TR/cssom-1/#the-cssstylerule-interface
#[derive(Debug, Clone, PartialEq)]
pub struct StyleRule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

impl StyleRule {
    pub fn get_matched_selectors(&self, dom_node: Rc<RefCell<DomNode>>) -> Option<Vec<Selector>> {
        // The matched selectors can be multiple, separated by commas.
        let mut matched_selectors = Vec::new();

        for selector in &self.selectors {
            if selector.matches(&dom_node) {
                matched_selectors.push(selector.clone());
            }
        }
        if !matched_selectors.is_empty() {
            Some(matched_selectors)
        } else {
            None
        }
    }
}

/// - https://www.w3.org/TR/css-syntax-3/#declaration
/// - https://www.w3.org/TR/cssom-1/#css-declarations
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Vec<ComponentValue>,
}

/// https://www.w3.org/TR/css-syntax-3/#component-value
#[derive(Clone, Debug, PartialEq)]
pub enum ComponentValue {
    PreservedToken(CssToken),
    Function {
        name: String,
        values: Vec<ComponentValue>,
    },
    SimpleBlock {
        associated_token: CssToken,
        values: Vec<ComponentValue>,
    },
}

/// https://www.w3.org/TR/css-syntax-3/#at-rules
#[derive(Debug, Clone, PartialEq)]
pub struct AtRule {
    pub name: String,
    pub prelude: Vec<ComponentValue>,
    pub block: Option<Box<Rule>>,
}
