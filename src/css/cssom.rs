use crate::css::selector::Selector;
use crate::css::tokenizer::CssToken;

/// https://www.w3.org/TR/cssom-1/#cssstylesheet
#[derive(Debug)]
pub struct StyleSheet {
    pub rules: Vec<Rule>,
    // pub parent_style_sheet: Option<Box<StyleSheet>>,
    // pub owner_rule: Option<Rule>,
}

/// A CSS document is a series of style rules and at-rules.
/// - https://www.w3.org/TR/css-syntax-3/#syntax-description
/// - https://www.w3.org/TR/cssom-1/#cssrule
#[derive(Debug, PartialEq)]
pub enum Rule {
    QualifiedRule(QualifiedRule),
    AtRule(AtRule),
}

/// https://www.w3.org/TR/css-syntax-3/#qualified-rule
pub type QualifiedRule = StyleRule;

/// - https://www.w3.org/TR/css-syntax-3/#style-rules
/// - https://www.w3.org/TR/cssom-1/#the-cssstylerule-interface
#[derive(Debug, PartialEq)]
pub struct StyleRule {
    // https://www.w3.org/TR/selectors-3/#grouping
    pub selectors: Vec<Selector>,

    pub declarations: Vec<Declaration>,
}

/// - https://www.w3.org/TR/css-syntax-3/#declaration
/// - https://www.w3.org/TR/cssom-1/#css-declarations
#[derive(Debug, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Vec<ComponentValue>,
}

/// https://www.w3.org/TR/css-syntax-3/#component-value
#[derive(Debug, PartialEq)]
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
