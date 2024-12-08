use std::collections::VecDeque;

use anyhow::{ensure, Result};

use crate::renderer::css::cssom::{
    AtRule, ComponentValue, Declaration, QualifiedRule, Rule, StyleSheet,
};
use crate::renderer::css::token::CssToken;
use crate::renderer::utils::TokenIterator;

use super::selector::SelectorParser;

#[derive(Debug)]
pub struct CssParser {
    input: TokenIterator<CssToken>,
}

impl CssParser {
    pub fn new(tokens: &[CssToken]) -> Self {
        Self {
            input: TokenIterator::new(tokens),
        }
    }

    /// Returns a stylesheet using the `Parse a stylesheet` entry point (normal parser entry point).
    /// https://www.w3.org/TR/css-syntax-3/#parse-stylesheet
    pub fn parse(&mut self) -> Result<StyleSheet> {
        Ok(StyleSheet::new(self.consume_list_of_rules()?))
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-list-of-rules
    fn consume_list_of_rules(&mut self) -> Result<Vec<Rule>> {
        let mut rules = Vec::new();

        loop {
            match self.input.next() {
                Some(CssToken::Whitespace) => continue,
                Some(CssToken::Eof) | None => return Ok(rules),
                Some(CssToken::Cdo | CssToken::Cdc) => {
                    unimplemented!();
                }
                Some(CssToken::AtKeyword(_)) => {
                    self.input.rewind(1);
                    rules.push(Rule::AtRule(self.consume_at_rule()?.unwrap()));
                }
                _ => {
                    self.input.rewind(1);
                    rules.push(Rule::QualifiedRule(self.consume_qualified_rule()?.unwrap()));
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-an-at-rule
    fn consume_at_rule(&mut self) -> Result<Option<AtRule>> {
        self.input.next();
        let CssToken::AtKeyword(name) = self.input.get_last_consumed().unwrap() else {
            unreachable!();
        };
        let mut at_rule = AtRule {
            name: name.clone(),
            prelude: Vec::new(),
            block: None,
        };

        loop {
            match self.input.next() {
                Some(CssToken::Semicolon) => return Ok(Some(at_rule)),
                Some(CssToken::Eof) => {
                    eprintln!("parse error in consume_at_rule");
                    return Ok(Some(at_rule));
                }
                Some(CssToken::OpenCurlyBrace) => {
                    while let Some(CssToken::Whitespace) = self.input.peek() {
                        self.input.next();
                    }
                    at_rule.block = Some(Box::new(Rule::QualifiedRule(
                        self.consume_qualified_rule()?.unwrap(),
                    )));
                    while let Some(CssToken::Whitespace) = self.input.peek() {
                        self.input.next();
                    }
                    ensure!(matches!(self.input.next(), Some(CssToken::CloseCurlyBrace)));
                    return Ok(Some(at_rule));
                }
                _ => {
                    self.input.rewind(1);
                    at_rule.prelude.push(self.consume_component_value());
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-a-qualified-rule
    fn consume_qualified_rule(&mut self) -> Result<Option<QualifiedRule>> {
        let mut qualified_rule = QualifiedRule {
            // The prelude of the qualified rule is parsed as a <selector-list>.
            selectors: Vec::new(),
            declarations: Vec::new(),
        };
        let mut selectors_buf = Vec::new();

        loop {
            match self.input.next() {
                Some(CssToken::Eof) | None => {
                    eprintln!("parse error in consume_qualified_rule");
                    return Ok(None);
                }
                Some(CssToken::OpenCurlyBrace) => {
                    // Here, the consume-list-of-declarations algorithm should be called on the
                    // result (a list of ComponentValue) after calling the consume-simple-block algorithm,
                    // but for the sake of simplicity, the consume-list-of-declarations algorithm is called from the beginning.
                    qualified_rule
                        .declarations
                        .extend(self.consume_list_of_declarations());

                    // Remove trailing whitespace tokens from the buffer, because
                    // the last whitespace tokens can't be parsed in the selector grammar.
                    while let Some(ComponentValue::PreservedToken(CssToken::Whitespace)) =
                        selectors_buf.last()
                    {
                        selectors_buf.pop();
                    }

                    // If the selector parsing fails, the the entire style rule is invalid, which means it must be ignored.
                    // This implementation stops parsing the CSS and returns an error in this case, instead of ignoring the rule.
                    qualified_rule
                        .selectors
                        .extend(SelectorParser::new(selectors_buf).parse()?);

                    return Ok(Some(qualified_rule));
                }
                _ => {
                    self.input.rewind(1);
                    selectors_buf.push(self.consume_component_value());
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-simple-block
    fn consume_simple_block(&mut self) -> ComponentValue {
        assert!(self.input.get_last_consumed().is_some_and(|t| matches!(
            t,
            CssToken::OpenCurlyBrace | CssToken::OpenParenthesis | CssToken::OpenSquareBracket
        )));
        let ending_token = match self.input.get_last_consumed().unwrap() {
            CssToken::OpenCurlyBrace => CssToken::CloseCurlyBrace,
            CssToken::OpenParenthesis => CssToken::CloseParenthesis,
            CssToken::OpenSquareBracket => CssToken::CloseSquareBracket,
            _ => {
                unreachable!();
            }
        };
        let mut block = ComponentValue::SimpleBlock {
            associated_token: self.input.get_last_consumed().unwrap().clone(),
            values: Vec::new(),
        };

        loop {
            match self.input.next() {
                Some(t) if t == ending_token => return block,
                Some(CssToken::Eof) | None => {
                    eprintln!("parse error in consume_simple_block");
                    return block;
                }
                _ => {
                    self.input.rewind(1);
                    if let ComponentValue::SimpleBlock { values, .. } = &mut block {
                        values.push(self.consume_component_value());
                    }
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-list-of-declarations
    fn consume_list_of_declarations(&mut self) -> Vec<Declaration> {
        assert!(self.input.get_last_consumed().is_some_and(|t| matches!(
            t,
            CssToken::OpenCurlyBrace | CssToken::OpenParenthesis | CssToken::OpenSquareBracket
        )));
        let mut declarations = Vec::new();

        // Partially follows the consume-simple-block algorithm.
        let ending_token = match self.input.get_last_consumed().unwrap() {
            CssToken::OpenCurlyBrace => CssToken::CloseCurlyBrace,
            CssToken::OpenParenthesis => CssToken::CloseParenthesis,
            CssToken::OpenSquareBracket => CssToken::CloseSquareBracket,
            _ => {
                unreachable!();
            }
        };

        loop {
            match self.input.next() {
                Some(t) if t == ending_token => return declarations,
                Some(CssToken::Whitespace) | Some(CssToken::Semicolon) => {}
                Some(CssToken::Eof) | None => return declarations,
                Some(CssToken::AtKeyword(_)) => {
                    unimplemented!();
                }
                Some(CssToken::Ident(_)) => {
                    let mut tmp_token_list = vec![ComponentValue::PreservedToken(
                        self.input.get_last_consumed().unwrap().clone(),
                    )];
                    while !matches!(
                        self.input.peek(),
                        Some(CssToken::Semicolon) | Some(CssToken::Eof)
                    ) {
                        tmp_token_list.push(self.consume_component_value());
                    }
                    if let Some(declaration) = Self::consume_declaration(tmp_token_list) {
                        declarations.push(declaration);
                    }
                }
                _ => {
                    eprintln!(
                        "parse error in consume_list_of_declarations: {:?}",
                        self.input.get_last_consumed()
                    );
                    self.input.rewind(1);
                    while !matches!(
                        self.input.peek(),
                        Some(CssToken::Semicolon) | Some(CssToken::Eof)
                    ) {
                        self.consume_component_value();
                    }
                }
            }
        }
    }

    /// This function is intended to be called for a given list of component values, not for default input stream.
    /// https://www.w3.org/TR/css-syntax-3/#consume-declaration
    fn consume_declaration(component_values: Vec<ComponentValue>) -> Option<Declaration> {
        let mut component_values = VecDeque::from(component_values);
        assert!(component_values
            .front()
            .is_some_and(|t| matches!(t, ComponentValue::PreservedToken(CssToken::Ident(_)))));
        let Some(ComponentValue::PreservedToken(CssToken::Ident(name))) =
            component_values.pop_front()
        else {
            unreachable!()
        };
        let mut declaration = Declaration {
            name,
            value: Vec::new(),
        };

        while component_values.front()
            == Some(&ComponentValue::PreservedToken(CssToken::Whitespace))
        {
            component_values.pop_front();
        }
        if component_values.front() != Some(&ComponentValue::PreservedToken(CssToken::Colon)) {
            eprintln!("parse error in consume_declaration");
            return None;
        } else {
            component_values.pop_front();
        }
        while component_values.front()
            == Some(&ComponentValue::PreservedToken(CssToken::Whitespace))
        {
            component_values.pop_front();
        }

        while component_values.front().is_some()
            && (component_values.front() != Some(&ComponentValue::PreservedToken(CssToken::Eof)))
        {
            let t = component_values.pop_front().unwrap();
            declaration.value.push(t);
        }

        if (declaration.value.len() >= 2)
            && declaration.value.get(declaration.value.len() - 2)
                == Some(&ComponentValue::PreservedToken(CssToken::Delim('!')))
        {
            if let Some(&ComponentValue::PreservedToken(CssToken::Ident(s))) =
                declaration.value.last().as_ref()
            {
                if s.eq_ignore_ascii_case("important") {
                    unimplemented!();
                }
            }
        }
        while declaration.value.last()
            == Some(&ComponentValue::PreservedToken(CssToken::Whitespace))
        {
            declaration.value.pop();
        }
        Some(declaration)
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-a-component-value
    fn consume_component_value(&mut self) -> ComponentValue {
        match self.input.next() {
            Some(
                CssToken::OpenParenthesis | CssToken::OpenSquareBracket | CssToken::OpenCurlyBrace,
            ) => self.consume_simple_block(),
            Some(CssToken::Function(_)) => self.consume_function(),
            Some(t) => ComponentValue::PreservedToken(t),
            None => ComponentValue::PreservedToken(CssToken::Eof),
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-function
    fn consume_function(&mut self) -> ComponentValue {
        assert!(self
            .input
            .get_last_consumed()
            .is_some_and(|t| matches!(t, CssToken::Function(_))));
        let CssToken::Function(name) = self.input.get_last_consumed().unwrap() else {
            unreachable!();
        };
        let mut function = ComponentValue::Function {
            name: name.clone(),
            values: Vec::new(),
        };

        loop {
            match self.input.next() {
                Some(CssToken::CloseParenthesis) => return function,
                Some(CssToken::Eof) | None => {
                    eprintln!("parse error in consume_function");
                    return function;
                }
                _ => {
                    self.input.rewind(1);
                    if let ComponentValue::Function { values, .. } = &mut function {
                        values.push(self.consume_component_value());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::css::cssom::{ComponentValue, Declaration, QualifiedRule, Rule};
    use crate::renderer::css::selector::{Combinator, Selector, SimpleSelector};
    use crate::renderer::css::token::{CssToken, CssTokenizer, NumericType};

    #[test]
    fn parse_simple_style() {
        let css = r#"
            h1 {
                color: red;
                grid-template-columns: 1fr 2fr;
            }
            h2 {
                color: blue;
            }
        "#;
        let style_sheet = CssParser::new(&CssTokenizer::new(css).tokenize().unwrap())
            .parse()
            .unwrap();
        let mut rules = style_sheet.rules.iter();
        assert_eq!(
            rules.next().unwrap(),
            &Rule::QualifiedRule(QualifiedRule {
                selectors: vec![Selector::Simple(vec![SimpleSelector::Type {
                    namespace_prefix: None,
                    name: "h1".to_string()
                }])],
                declarations: vec![
                    Declaration {
                        name: "color".to_string(),
                        value: vec![ComponentValue::PreservedToken(CssToken::Ident(
                            "red".to_string()
                        ))],
                    },
                    Declaration {
                        name: "grid-template-columns".to_string(),
                        value: vec![
                            ComponentValue::PreservedToken(CssToken::Dimension(
                                NumericType::Integer(1),
                                "fr".to_string()
                            )),
                            ComponentValue::PreservedToken(CssToken::Whitespace),
                            ComponentValue::PreservedToken(CssToken::Dimension(
                                NumericType::Integer(2),
                                "fr".to_string()
                            )),
                        ],
                    },
                ],
            })
        );
    }

    #[test]
    fn parse_simple_style_with_complex_selector() {
        let css = r#"
            h1, h2, h3 {
                color: red;
            }
            #myId > .myClass + div > h1[title="hello"] {
                color: blue;
                font-size: 16px;
            }
        "#;
        let style_sheet = CssParser::new(&CssTokenizer::new(css).tokenize().unwrap())
            .parse()
            .unwrap();
        let mut rules = style_sheet.rules.iter();
        assert_eq!(
            rules.next().unwrap(),
            &Rule::QualifiedRule(QualifiedRule {
                selectors: vec![
                    Selector::Simple(vec![SimpleSelector::Type {
                        namespace_prefix: None,
                        name: "h1".to_string()
                    }]),
                    Selector::Simple(vec![SimpleSelector::Type {
                        namespace_prefix: None,
                        name: "h2".to_string()
                    }]),
                    Selector::Simple(vec![SimpleSelector::Type {
                        namespace_prefix: None,
                        name: "h3".to_string()
                    }]),
                ],
                declarations: vec![Declaration {
                    name: "color".to_string(),
                    value: vec![ComponentValue::PreservedToken(CssToken::Ident(
                        "red".to_string()
                    ))],
                }],
            })
        );
        assert_eq!(
            rules.next().unwrap(),
            &Rule::QualifiedRule(QualifiedRule {
                selectors: vec![Selector::Complex(
                    Box::new(Selector::Simple(vec![SimpleSelector::Id(
                        "myId".to_string()
                    ),])),
                    Combinator::GreaterThan,
                    Box::new(Selector::Complex(
                        Box::new(Selector::Simple(vec![SimpleSelector::Class(
                            "myClass".to_string()
                        ),])),
                        Combinator::Plus,
                        Box::new(Selector::Complex(
                            Box::new(Selector::Simple(vec![SimpleSelector::Type {
                                namespace_prefix: None,
                                name: "div".to_string()
                            },])),
                            Combinator::GreaterThan,
                            Box::new(Selector::Simple(vec![
                                SimpleSelector::Type {
                                    namespace_prefix: None,
                                    name: "h1".to_string(),
                                },
                                SimpleSelector::Attribute {
                                    namespace_prefix: None,
                                    name: "title".to_string(),
                                    op: Some("=".to_string()),
                                    value: Some("hello".to_string()),
                                },
                            ]))
                        ))
                    ))
                )],
                declarations: vec![
                    Declaration {
                        name: "color".to_string(),
                        value: vec![ComponentValue::PreservedToken(CssToken::Ident(
                            "blue".to_string()
                        ))],
                    },
                    Declaration {
                        name: "font-size".to_string(),
                        value: vec![ComponentValue::PreservedToken(CssToken::Dimension(
                            NumericType::Integer(16),
                            "px".to_string()
                        ))],
                    },
                ],
            })
        );
    }
}
