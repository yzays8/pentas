use std::collections::VecDeque;

use anyhow::Result;

use crate::css::cssom::{ComponentValue, Declaration, QualifiedRule, Rule, StyleSheet};
use crate::css::selector::SelectorParser;
use crate::css::tokenizer::CssToken;

#[derive(Debug)]
pub struct CssParser {
    input: Vec<CssToken>,
    current_pos: usize,
    current_token: Option<CssToken>,
}

impl CssParser {
    pub fn new(tokens: Vec<CssToken>) -> Self {
        Self {
            input: tokens,
            current_pos: 0,
            current_token: None,
        }
    }

    /// This is `Parse a stylesheet` entry point (normal parser entry point).
    /// https://www.w3.org/TR/css-syntax-3/#parse-stylesheet
    pub fn parse(&mut self) -> Result<StyleSheet> {
        Ok(StyleSheet::new(self.consume_list_of_rules()?))
    }

    /// Returns the next token in the input stream and advances the current position.
    fn consume_token(&mut self) -> CssToken {
        let token = if let Some(token) = self.input.get(self.current_pos) {
            token
        } else {
            // Whenever the list of tokens is empty, the next input token is always an <EOF-token>.
            return CssToken::Eof;
        };
        self.current_token = Some(token.clone());
        self.current_pos += 1;
        token.to_owned()
    }

    /// Returns the next token in the input stream without consuming it.
    fn peek_token(&self) -> CssToken {
        if let Some(token) = self.input.get(self.current_pos) {
            token.to_owned()
        } else {
            // Whenever the list of tokens is empty, the next input token is always an <EOF-token>.
            CssToken::Eof
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#reconsume-the-current-input-token
    fn recomsume(&mut self) {
        self.current_pos -= 1;
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-list-of-rules
    fn consume_list_of_rules(&mut self) -> Result<Vec<Rule>> {
        let mut rules = Vec::new();

        loop {
            match self.consume_token() {
                CssToken::Whitespace => continue,
                CssToken::Eof => return Ok(rules),
                CssToken::Cdo | CssToken::Cdc => {
                    unimplemented!();
                }
                CssToken::AtKeyword(_) => {
                    unimplemented!();
                }
                _ => {
                    self.recomsume();
                    rules.push(Rule::QualifiedRule(self.consume_qualified_rule()?.unwrap()));
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
            match self.consume_token() {
                CssToken::Eof => {
                    eprintln!("parse error in consume_qualified_rule");
                    return Ok(None);
                }
                CssToken::OpenBrace => {
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
                    self.recomsume();
                    selectors_buf.push(self.consume_component_value());
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-simple-block
    fn consume_simple_block(&mut self) -> ComponentValue {
        assert!(self.current_token.as_ref().is_some_and(|t| matches!(
            t,
            CssToken::OpenBrace | CssToken::OpenParenthesis | CssToken::OpenSquareBracket
        )));
        let ending_token = match self.current_token.as_ref().unwrap() {
            CssToken::OpenBrace => CssToken::CloseBrace,
            CssToken::OpenParenthesis => CssToken::CloseParenthesis,
            CssToken::OpenSquareBracket => CssToken::CloseSquareBracket,
            _ => {
                unreachable!();
            }
        };
        let mut block = ComponentValue::SimpleBlock {
            associated_token: self.current_token.as_ref().unwrap().clone(),
            values: Vec::new(),
        };

        loop {
            match self.consume_token() {
                t if t == ending_token => return block,
                CssToken::Eof => {
                    eprintln!("parse error in consume_simple_block");
                    return block;
                }
                _ => {
                    self.recomsume();
                    if let ComponentValue::SimpleBlock { values, .. } = &mut block {
                        values.push(self.consume_component_value());
                    }
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-list-of-declarations
    fn consume_list_of_declarations(&mut self) -> Vec<Declaration> {
        assert!(self.current_token.as_ref().is_some_and(|t| matches!(
            t,
            CssToken::OpenBrace | CssToken::OpenParenthesis | CssToken::OpenSquareBracket
        )));
        let mut declarations = Vec::new();

        // Partially follows the consume-simple-block algorithm.
        let ending_token = match self.current_token.as_ref().unwrap() {
            CssToken::OpenBrace => CssToken::CloseBrace,
            CssToken::OpenParenthesis => CssToken::CloseParenthesis,
            CssToken::OpenSquareBracket => CssToken::CloseSquareBracket,
            _ => {
                unreachable!();
            }
        };

        loop {
            match self.consume_token() {
                t if t == ending_token => return declarations,
                CssToken::Whitespace | CssToken::Semicolon => {}
                CssToken::Eof => return declarations,
                CssToken::AtKeyword(_) => {
                    unimplemented!();
                }
                CssToken::Ident(_) => {
                    let mut tmp_token_list = vec![ComponentValue::PreservedToken(
                        self.current_token.clone().unwrap(),
                    )];
                    while (self.peek_token() != CssToken::Semicolon)
                        && (self.peek_token() != CssToken::Eof)
                    {
                        tmp_token_list.push(self.consume_component_value());
                    }
                    if let Some(declaration) = self.consume_declaration(tmp_token_list) {
                        declarations.push(declaration);
                    }
                }
                _ => {
                    eprintln!(
                        "parse error in consume_list_of_declarations: {:?}",
                        self.current_token
                    );
                    self.recomsume();
                    while (self.peek_token() != CssToken::Semicolon)
                        && (self.peek_token() != CssToken::Eof)
                    {
                        self.consume_component_value();
                    }
                }
            }
        }
    }

    /// This function is intended to be called for a given list of component values, not for default input stream.
    /// https://www.w3.org/TR/css-syntax-3/#consume-declaration
    fn consume_declaration(
        &mut self,
        component_values: Vec<ComponentValue>,
    ) -> Option<Declaration> {
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
        let token = self.consume_token();
        if let CssToken::OpenParenthesis | CssToken::OpenSquareBracket | CssToken::OpenBrace = token
        {
            self.consume_simple_block()
        } else if let CssToken::Function(_) = token {
            self.consume_function()
        } else {
            ComponentValue::PreservedToken(token)
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-function
    fn consume_function(&mut self) -> ComponentValue {
        assert!(self
            .current_token
            .as_ref()
            .is_some_and(|t| matches!(t, CssToken::Function(_))));
        let CssToken::Function(name) = self.current_token.as_ref().unwrap() else {
            unreachable!();
        };
        let mut function = ComponentValue::Function {
            name: name.clone(),
            values: Vec::new(),
        };

        loop {
            match &self.consume_token() {
                CssToken::CloseParenthesis => return function,
                CssToken::Eof => {
                    eprintln!("parse error in consume_function");
                    return function;
                }
                _ => {
                    self.recomsume();
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
    use crate::css::cssom::*;
    use crate::css::selector::*;
    use crate::css::tokenizer::*;

    #[test]
    fn test_parse1() {
        let css = r#"
            h1 {
                color: red;
                grid-template-columns: 1fr 2fr;
            }
            h2 {
                color: blue;
            }
        "#;
        let mut parser = CssParser::new(CssTokenizer::new(css).tokenize().unwrap());
        let style_sheet = parser.parse().unwrap();
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
    fn test_parse2() {
        let css = r#"
            h1, h2, h3 {
                color: red;
            }
            #myId > .myClass + div > h1[title="hello"] {
                color: blue;
                font-size: 16px;
            }
        "#;
        let mut parser = CssParser::new(CssTokenizer::new(css).tokenize().unwrap());
        let style_sheet = parser.parse().unwrap();
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
