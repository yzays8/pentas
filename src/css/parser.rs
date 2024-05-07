use std::collections::VecDeque;

use crate::css::cssom::{
    ComponentValue, Declaration, QualifiedRule, Rule, Selector, SimpleSelector, StyleSheet,
};
use crate::css::tokenizer::Token;

#[derive(Debug)]
pub struct Parser {
    need_reconsume: bool,
    tokens: Vec<Token>,
    current_pos: usize,
    current_token: Option<Token>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            need_reconsume: false,
            tokens,
            current_pos: 0,
            current_token: None,
        }
    }

    pub fn parse(&mut self) -> StyleSheet {
        StyleSheet {
            rules: self.consume_list_of_rules(),
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-list-of-rules
    fn consume_list_of_rules(&mut self) -> Vec<Rule> {
        let mut rules = Vec::new();

        loop {
            match self.consume_token() {
                Token::Whitespace => continue,
                Token::Eof => return rules,
                Token::Cdo | Token::Cdc => {
                    unimplemented!();
                }
                Token::AtKeyword(_) => {
                    unimplemented!();
                }
                _ => {
                    self.need_reconsume = true;
                    rules.push(Rule::QualifiedRule(self.consume_qualified_rule().unwrap()));
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-a-qualified-rule
    fn consume_qualified_rule(&mut self) -> Option<QualifiedRule> {
        let mut qualified_rule = QualifiedRule {
            // A prelude for style rules is a selector.
            // https://www.w3.org/TR/css-syntax-3/#syntax-description
            selectors: Vec::new(),
            declarations: Vec::new(),
        };

        loop {
            match self.consume_token() {
                Token::Eof => {
                    eprintln!("parse error in consume_qualified_rule");
                    return None;
                }
                Token::OpenBrace => {
                    qualified_rule
                        .declarations
                        .extend(self.consume_list_of_declarations());
                    return Some(qualified_rule);
                }
                _ => {
                    // todo: need to parse selector
                    self.need_reconsume = true;
                    let component_val = self.consume_component_value();
                    qualified_rule.selectors.push(match component_val {
                        ComponentValue::PreservedToken(Token::Ident(s)) => {
                            Selector::Simple(SimpleSelector::Type(s.to_string()))
                        }
                        _ => Selector::Simple(SimpleSelector::Type("".to_string())),
                    });
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-simple-block
    fn consume_simple_block(&mut self) -> ComponentValue {
        let ending_token = match self.current_token.as_ref().unwrap() {
            Token::OpenBrace => Token::CloseBrace,
            Token::OpenParenthesis => Token::CloseParenthesis,
            Token::OpenSquareBracket => Token::CloseSquareBracket,
            _ => {
                unreachable!();
            }
        };
        let mut block = ComponentValue::SimpleBlock {
            associated_token: self.current_token.clone().unwrap(),
            values: Vec::new(),
        };

        loop {
            match self.consume_token() {
                t if t == ending_token => return block,
                Token::Eof => {
                    eprintln!("parse error in consume_simple_block");
                    return block;
                }
                _ => {
                    self.need_reconsume = true;
                    if let ComponentValue::SimpleBlock { values, .. } = &mut block {
                        values.push(self.consume_component_value());
                    }
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-list-of-declarations
    fn consume_list_of_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();

        // Partially follows the consume simple block algorithm.
        // https://www.w3.org/TR/css-syntax-3/#consume-simple-block
        let ending_token = match self.current_token.as_ref().unwrap() {
            Token::OpenBrace => Token::CloseBrace,
            Token::OpenParenthesis => Token::CloseParenthesis,
            Token::OpenSquareBracket => Token::CloseSquareBracket,
            _ => {
                unreachable!();
            }
        };

        loop {
            match self.consume_token() {
                t if t == ending_token => return declarations,
                Token::Whitespace | Token::Semicolon => continue,
                Token::Eof => return declarations,
                Token::AtKeyword(_) => {
                    unimplemented!();
                }
                Token::Ident(_) => {
                    // todo
                    let mut tmp_token_list = vec![self.current_token.clone().unwrap()];
                    while (self.peek_token() != Token::Semicolon)
                        && (self.peek_token() != Token::Eof)
                    {
                        tmp_token_list.push(
                            if let ComponentValue::PreservedToken(token) =
                                self.consume_component_value()
                            {
                                token
                            } else {
                                unreachable!();
                            },
                        );
                    }
                    if let Some(declaration) =
                        self.consume_declaration(VecDeque::from(tmp_token_list))
                    {
                        declarations.push(declaration);
                    }
                }
                _ => {
                    eprintln!(
                        "parse error in consume_list_of_declarations: {:?}",
                        self.current_token
                    );
                    self.need_reconsume = true;
                    while (self.peek_token() != Token::Semicolon)
                        && (self.peek_token() != Token::Eof)
                    {
                        self.consume_component_value();
                    }
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-declaration
    fn consume_declaration(&mut self, mut tokens: VecDeque<Token>) -> Option<Declaration> {
        let Some(Token::Ident(name)) = tokens.pop_front() else {
            unreachable!();
        };
        let mut declaration = Declaration {
            name,
            value: Vec::new(),
        };

        while tokens.front() == Some(&Token::Whitespace) {
            tokens.pop_front();
        }
        if tokens.front() != Some(&Token::Colon) {
            eprintln!("parse error in consume_declaration");
            return None;
        } else {
            tokens.pop_front();
        }
        while tokens.front() == Some(&Token::Whitespace) {
            tokens.pop_front();
        }
        // todo
        while tokens.front().is_some() && (tokens.front() != Some(&Token::Eof)) {
            let t = tokens.pop_front().unwrap();
            let c = if let Token::OpenParenthesis | Token::OpenSquareBracket | Token::OpenBrace = t
            {
                unimplemented!();
            } else if let Token::Function(_) = t {
                unimplemented!();
            } else {
                ComponentValue::PreservedToken(t)
            };
            declaration.value.push(c);
        }

        if (declaration.value.len() >= 2)
            && declaration.value.get(declaration.value.len() - 2)
                == Some(&ComponentValue::PreservedToken(Token::Delim('!')))
        {
            if let Some(&ComponentValue::PreservedToken(Token::Ident(s))) =
                declaration.value.last().as_ref()
            {
                if s.eq_ignore_ascii_case("important") {
                    unimplemented!();
                }
            }
        }
        while declaration.value.last() == Some(&ComponentValue::PreservedToken(Token::Whitespace)) {
            declaration.value.pop();
        }
        Some(declaration)
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-a-component-value
    fn consume_component_value(&mut self) -> ComponentValue {
        let token = self.consume_token();
        if let Token::OpenParenthesis | Token::OpenSquareBracket | Token::OpenBrace = token {
            self.consume_simple_block()
        } else if let Token::Function(_) = token {
            self.consume_function()
        } else {
            ComponentValue::PreservedToken(token)
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-function
    fn consume_function(&mut self) -> ComponentValue {
        let Token::Function(name) = self.current_token.clone().unwrap() else {
            unreachable!();
        };
        let mut function = ComponentValue::Function {
            name,
            values: Vec::new(),
        };
        loop {
            match &self.consume_token() {
                Token::CloseParenthesis => return function,
                Token::Eof => {
                    eprintln!("parse error in consume_function");
                    return function;
                }
                _ => {
                    self.need_reconsume = true;
                    if let ComponentValue::Function { values, .. } = &mut function {
                        values.push(self.consume_component_value());
                    }
                }
            }
        }
    }

    fn consume_token(&mut self) -> Token {
        if self.need_reconsume {
            self.need_reconsume = false;
            self.current_token.clone().unwrap()
        } else {
            let token = if let Some(token) = self.tokens.get(self.current_pos) {
                token
            } else {
                return Token::Eof;
            };
            self.current_token = Some(token.clone());
            self.current_pos += 1;
            token.to_owned()
        }
    }

    fn peek_token(&self) -> Token {
        if let Some(token) = self.tokens.get(self.current_pos) {
            token.to_owned()
        } else {
            Token::Eof
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::tokenizer::Tokenizer;

    #[test]
    fn test_parser() {
        let css = r#"
            h1 {
                color: red;
                grid-template-columns: 1fr 2fr;
            }
            h2 {
                color: blue;
            }
        "#;
        let tokens = Tokenizer::new(css).tokenize();
        let mut parser = Parser::new(tokens.unwrap());
        let style_sheet = parser.parse();
        println!("{:#?}", style_sheet);
    }
}
