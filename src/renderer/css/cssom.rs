use std::cell::RefCell;
use std::rc::Rc;

use crate::renderer::css::selector::Selector;
use crate::renderer::css::token::CssToken;
use crate::renderer::html::dom::DomNode;

/// https://www.w3.org/TR/cssom-1/#cssstylesheet
#[derive(Debug)]
pub struct StyleSheet {
    pub rules: Vec<Rule>,
    // pub parent_style_sheet: Option<Box<StyleSheet>>,
    // pub owner_rule: Option<Rule>,
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
#[derive(Debug, PartialEq)]
pub enum Rule {
    QualifiedRule(QualifiedRule),
    // AtRule(AtRule),
}

impl Rule {
    pub fn get_matched_selectors(&self, dom_node: Rc<RefCell<DomNode>>) -> Option<Vec<Selector>> {
        match self {
            Rule::QualifiedRule(rule) => rule.get_matched_selectors(dom_node),
            // Only QualifiedRule is supported for now.
        }
    }
}

/// https://www.w3.org/TR/css-syntax-3/#qualified-rule
pub type QualifiedRule = StyleRule;

/// - https://www.w3.org/TR/css-syntax-3/#style-rules
/// - https://www.w3.org/TR/cssom-1/#the-cssstylerule-interface
#[derive(Debug, PartialEq)]
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

/// - https://www.w3.org/TR/css-syntax-3/#at-rules
/// - https://www.w3.org/TR/cssom-1/#the-cssimportrule-interface and subsequent sections for at-rule interfaces
#[derive(Debug, PartialEq)]
pub enum AtRule {
    // unimplemented
}
