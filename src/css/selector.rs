use std::collections::VecDeque;

use anyhow::{bail, Ok, Result};

use crate::css::cssom::ComponentValue;
use crate::css::tokenizer::Token;

/// - https://www.w3.org/TR/selectors-3/#simple-selectors
/// - https://www.w3.org/TR/selectors-3/#grammar
#[derive(Debug, PartialEq)]
pub enum SimpleSelector {
    Type {
        namespace_prefix: Option<String>,
        name: String,
    },
    Universal(Option<String>), // <namespace prefix>
    Attribute {
        namespace_prefix: Option<String>,
        name: String,
        op: Option<String>,
        value: Option<String>,
    },
    Class(String),
    Id(String),
    PseudoClass,
}

#[derive(Debug, PartialEq)]
pub enum Combinator {
    Whitespace,
    GreaterThan,
    Plus,
    Tilde,
}

/// https://www.w3.org/TR/selectors-3/#selector-syntax
#[derive(Debug, PartialEq)]
pub enum Selector {
    Simple(Vec<SimpleSelector>),

    // The left value can have a Complex selector in this data structure,
    // but it must be a Simple selector because of the right associativity of the selector.
    // https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_selectors/Selector_structure#complex_selector
    Complex(Box<Selector>, Combinator, Box<Selector>),
}

#[derive(Debug)]
pub struct SelectorParser {
    input: VecDeque<ComponentValue>,
}

impl SelectorParser {
    pub fn new(input: Vec<ComponentValue>) -> Self {
        Self {
            input: input.into(),
        }
    }

    fn consume(&mut self) -> Option<ComponentValue> {
        self.input.pop_front()
    }

    /// https://www.w3.org/TR/selectors-3/#w3cselgrammar
    pub fn parse(&mut self) -> Result<Vec<Selector>> {
        self.parse_selectors_group()
    }

    // selectors_group
    //   : selector [ COMMA S* selector ]*
    //   ;
    fn parse_selectors_group(&mut self) -> Result<Vec<Selector>> {
        let mut selectors = Vec::new();
        selectors.push(self.parse_selector()?);
        loop {
            match self.consume() {
                Some(ComponentValue::PreservedToken(Token::Comma)) => {
                    while let Some(ComponentValue::PreservedToken(Token::Whitespace)) =
                        self.input.front()
                    {
                        self.input.pop_front();
                    }
                    selectors.push(self.parse_selector()?);
                }
                None => break,
                t => {
                    println!("{:#?}", self.input);
                    bail!("Unexpected token when parsing CSS selectors in parse_selectors_group: {:?}", t);
                }
            }
        }
        Ok(selectors)
    }

    // selector
    //   : simple_selector_sequence [ combinator simple_selector_sequence ]*
    //   ;
    fn parse_selector(&mut self) -> Result<Selector> {
        let simple = Selector::Simple(self.parse_simple_selector_seq()?);
        if let Some(combinator) = self.parse_combinator() {
            Ok(Selector::Complex(
                Box::new(simple),
                combinator,
                Box::new(self.parse_selector()?),
            ))
        } else {
            Ok(simple)
        }
    }

    // combinator
    //   /* combinators can be surrounded by whitespace */
    //   : PLUS S* | GREATER S* | TILDE S* | S+
    //   ;
    fn parse_combinator(&mut self) -> Option<Combinator> {
        let mut is_detected_space = false;
        while let Some(ComponentValue::PreservedToken(Token::Whitespace)) = self.input.front() {
            self.input.pop_front();
            is_detected_space = true;
        }

        match self.input.front() {
            Some(ComponentValue::PreservedToken(Token::Delim('+'))) => {
                self.input.pop_front();
                while let Some(ComponentValue::PreservedToken(Token::Whitespace)) =
                    self.input.front()
                {
                    self.input.pop_front();
                }
                Some(Combinator::Plus)
            }
            Some(ComponentValue::PreservedToken(Token::Delim('>'))) => {
                self.input.pop_front();
                while let Some(ComponentValue::PreservedToken(Token::Whitespace)) =
                    self.input.front()
                {
                    self.input.pop_front();
                }
                Some(Combinator::GreaterThan)
            }
            Some(ComponentValue::PreservedToken(Token::Delim('~'))) => {
                self.input.pop_front();
                while let Some(ComponentValue::PreservedToken(Token::Whitespace)) =
                    self.input.front()
                {
                    self.input.pop_front();
                }
                Some(Combinator::Tilde)
            }
            _ => {
                if is_detected_space {
                    Some(Combinator::Whitespace)
                } else {
                    None
                }
            }
        }
    }

    // simple_selector_sequence
    //   : [ type_selector | universal ]
    //     [ HASH | class | attrib | pseudo | negation ]*
    //   | [ HASH | class | attrib | pseudo | negation ]+
    //   ;
    fn parse_simple_selector_seq(&mut self) -> Result<Vec<SimpleSelector>> {
        // todo: parse pseudo and negation

        let mut selector_seq = Vec::new();

        // Check that the tokens match [ HASH | class | attrib | pseudo | negation ]+
        match self.input.front() {
            Some(ComponentValue::PreservedToken(Token::Hash(..))) => {
                while let Some(ComponentValue::PreservedToken(Token::Hash(s, ..))) =
                    self.input.front()
                {
                    let s = s.clone();
                    self.input.pop_front();
                    selector_seq.push(SimpleSelector::Id(s.to_string()));
                }
            }
            Some(ComponentValue::PreservedToken(Token::Delim('.'))) => {
                while let Some(ComponentValue::PreservedToken(Token::Delim('.'))) =
                    self.input.front()
                {
                    selector_seq.push(self.parse_class()?);
                }
            }
            Some(ComponentValue::SimpleBlock { .. }) => {
                while let Some(ComponentValue::SimpleBlock {
                    associated_token: t,
                    ..
                }) = self.input.front()
                {
                    if t != &Token::OpenSquareBracket {
                        bail!("Expected \"[\" but found {:?} when parsing CSS selectors in parse_simple_selector_seq", self.input.front());
                    }
                    selector_seq.push(self.parse_attrib()?);
                }
            }
            _ => {}
        }

        // If the tokens don't match [ HASH | class | attrib | pseudo | negation ]+, selector_seq is left empty.
        // Check that the tokens match [ type_selector | universal ] [ HASH | class | attrib | pseudo | negation ]*
        if selector_seq.is_empty() {
            let prefix = self.parse_namespace_prefix();
            if self.input.front() == Some(&ComponentValue::PreservedToken(Token::Delim('*'))) {
                // universal
                self.input.pop_front();
                selector_seq.push(SimpleSelector::Universal(prefix));
            } else {
                // type_selector
                selector_seq.push(SimpleSelector::Type {
                    namespace_prefix: prefix,
                    name: self.parse_element_name()?,
                });
            }

            match self.input.front() {
                Some(ComponentValue::PreservedToken(Token::Hash(..))) => {
                    while let Some(ComponentValue::PreservedToken(Token::Hash(s, ..))) =
                        self.input.front()
                    {
                        let s = s.clone();
                        self.input.pop_front();
                        selector_seq.push(SimpleSelector::Id(s.to_string()));
                    }
                }
                Some(ComponentValue::PreservedToken(Token::Delim('.'))) => {
                    while let Some(ComponentValue::PreservedToken(Token::Delim('.'))) =
                        self.input.front()
                    {
                        selector_seq.push(self.parse_class()?);
                    }
                }
                Some(ComponentValue::SimpleBlock { .. }) => {
                    while let Some(ComponentValue::SimpleBlock {
                        associated_token: t,
                        ..
                    }) = self.input.front()
                    {
                        if t != &Token::OpenSquareBracket {
                            bail!("Expected \"[\" but found {:?} when parsing CSS selectors in parse_simple_selector_seq", self.input.front());
                        }
                        selector_seq.push(self.parse_attrib()?);
                    }
                }
                _ => {}
            }
        }

        Ok(selector_seq)
    }

    // type_selector
    //   : [ namespace_prefix ]? element_name
    //   ;
    fn parse_type_selector(&mut self) -> Result<SimpleSelector> {
        Ok(SimpleSelector::Type {
            namespace_prefix: self.parse_namespace_prefix(),
            name: self.parse_element_name()?,
        })
    }

    // namespace_prefix
    //   : [ IDENT | '*' ]? '|'
    //   ;
    fn parse_namespace_prefix(&mut self) -> Option<String> {
        match self.input.front() {
            Some(ComponentValue::PreservedToken(Token::Delim('|'))) => {
                self.input.pop_front();
                Some("".to_string())
            }
            Some(ComponentValue::PreservedToken(Token::Ident(s))) => {
                if self.input.get(1) == Some(&ComponentValue::PreservedToken(Token::Delim('|'))) {
                    let s = s.clone();
                    self.input.pop_front();
                    self.input.pop_front();
                    Some(s)
                } else {
                    None
                }
            }
            Some(ComponentValue::PreservedToken(Token::Delim('*'))) => {
                if self.input.get(1) == Some(&ComponentValue::PreservedToken(Token::Delim('|'))) {
                    self.input.pop_front();
                    self.input.pop_front();
                    Some("*".to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // element_name
    //   : IDENT
    //   ;
    fn parse_element_name(&mut self) -> Result<String> {
        if let Some(ComponentValue::PreservedToken(Token::Ident(s))) = self.input.pop_front() {
            Ok(s)
        } else {
            bail!(
                "Expected ident but found {:?} when parsing CSS selectors in parse_element_name",
                self.input.front()
            );
        }
    }

    // universal
    //   : [ namespace_prefix ]? '*'
    //   ;
    fn parse_universal(&mut self) -> Result<SimpleSelector> {
        let prefix = self.parse_namespace_prefix();
        if self.input.pop_front() == Some(ComponentValue::PreservedToken(Token::Delim('*'))) {
            Ok(SimpleSelector::Universal(prefix))
        } else {
            bail!(
                "Expected \"*\" but found {:?} when parsing CSS selectors in parse_universal",
                self.input.front()
            );
        }
    }

    // class
    //   : '.' IDENT
    //   ;
    fn parse_class(&mut self) -> Result<SimpleSelector> {
        if self.input.pop_front() == Some(ComponentValue::PreservedToken(Token::Delim('.'))) {
            Ok(SimpleSelector::Class(self.parse_element_name()?))
        } else {
            bail!(
                "Expected \".\" but found {:?} when parsing CSS selectors in parse_class",
                self.input.front()
            );
        }
    }

    // attrib
    //   : '[' S* [ namespace_prefix ]? IDENT S*
    //         [ [ PREFIXMATCH |
    //             SUFFIXMATCH |
    //             SUBSTRINGMATCH |
    //             '=' |
    //             INCLUDES |
    //             DASHMATCH ] S* [ IDENT | STRING ] S*
    //         ]? ']'
    //   ;
    fn parse_attrib(&mut self) -> Result<SimpleSelector> {
        if let Some(ComponentValue::SimpleBlock {
            associated_token: t,
            values,
        }) = self.input.pop_front()
        {
            if t != Token::OpenSquareBracket {
                bail!(
                    "Expected \"[\" but found {:?} when parsing CSS selectors in parse_attrib",
                    self.input.front()
                );
            }

            let mut values: VecDeque<ComponentValue> = VecDeque::from(values);

            while let Some(ComponentValue::PreservedToken(Token::Whitespace)) = values.front() {
                values.pop_front();
            }

            // Parse prefix
            let prefix = match self.input.front() {
                Some(ComponentValue::PreservedToken(Token::Delim('|'))) => {
                    self.input.pop_front();
                    Some("".to_string())
                }
                Some(ComponentValue::PreservedToken(Token::Ident(s))) => {
                    if self.input.get(1) == Some(&ComponentValue::PreservedToken(Token::Delim('|')))
                    {
                        let s = s.clone();
                        self.input.pop_front();
                        self.input.pop_front();
                        Some(s)
                    } else {
                        None
                    }
                }
                Some(ComponentValue::PreservedToken(Token::Delim('*'))) => {
                    if self.input.get(1) == Some(&ComponentValue::PreservedToken(Token::Delim('|')))
                    {
                        self.input.pop_front();
                        self.input.pop_front();
                        Some("*".to_string())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            let Some(ComponentValue::PreservedToken(Token::Ident(name))) = values.pop_front()
            else {
                bail!(
                    "Expected ident but found {:?} when parsing CSS selectors in parse_attrib",
                    values.front()
                );
            };
            while let Some(ComponentValue::PreservedToken(Token::Whitespace)) = values.front() {
                values.pop_front();
            }

            match (values.front(), values.front()) {
                (Some(ComponentValue::PreservedToken(Token::Delim(c1))), None)
                | (
                    Some(ComponentValue::PreservedToken(Token::Delim(c1))),
                    Some(ComponentValue::PreservedToken(Token::Delim('='))),
                ) => {
                    let c1 = *c1;
                    let op = if c1 == '=' {
                        values.pop_front();
                        "=".to_string()
                    } else if let '^' | '$' | '*' | '~' | '|' = c1 {
                        values.pop_front();
                        values.pop_front();
                        format!("{}=", c1)
                    } else {
                        bail!(
                            "Unexpected token when parsing CSS selectors in parse_attrib: {:?}",
                            values.front()
                        );
                    };

                    while let Some(ComponentValue::PreservedToken(Token::Whitespace)) =
                        values.front()
                    {
                        values.pop_front();
                    }

                    let value = match values.pop_front() {
                        Some(ComponentValue::PreservedToken(Token::Ident(s))) => Some(s),
                        Some(ComponentValue::PreservedToken(Token::String(s))) => Some(s),
                        _ => {
                            bail!("Expected ident or string but found {:?} when parsing CSS selectors in parse_attrib", values.front());
                        }
                    };

                    while let Some(ComponentValue::PreservedToken(Token::Whitespace)) =
                        values.front()
                    {
                        values.pop_front();
                    }

                    Ok(SimpleSelector::Attribute {
                        namespace_prefix: prefix,
                        name,
                        op: Some(op),
                        value,
                    })
                }
                (None, None) => Ok(SimpleSelector::Attribute {
                    namespace_prefix: prefix,
                    name,
                    op: None,
                    value: None,
                }),
                _ => bail!(
                    "Unexpected token when parsing CSS selectors in parse_attrib: {:?}",
                    values.front()
                ),
            }
        } else {
            bail!(
                "Expected simple block but found {:?} when parsing CSS selectors in parse_attrib",
                self.input.front()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_selector1() {
        // div > p

        let input = vec![
            ComponentValue::PreservedToken(Token::Ident("div".to_string())),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Delim('>')),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Ident("p".to_string())),
        ];
        let mut parser = SelectorParser::new(input);
        assert_eq!(
            parser.parse_selectors_group().unwrap(),
            vec![Selector::Complex(
                Box::new(Selector::Simple(vec![SimpleSelector::Type {
                    namespace_prefix: None,
                    name: "div".to_string(),
                }])),
                Combinator::GreaterThan,
                Box::new(Selector::Simple(vec![SimpleSelector::Type {
                    namespace_prefix: None,
                    name: "p".to_string(),
                }]))
            )]
        );
    }

    #[test]
    fn test_parse_selector2() {
        // div > p, a + b

        let input = vec![
            ComponentValue::PreservedToken(Token::Ident("div".to_string())),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Delim('>')),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Ident("p".to_string())),
            ComponentValue::PreservedToken(Token::Comma),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Ident("a".to_string())),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Delim('+')),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Ident("b".to_string())),
        ];
        let mut parser = SelectorParser::new(input);
        assert_eq!(
            parser.parse_selectors_group().unwrap(),
            vec![
                Selector::Complex(
                    Box::new(Selector::Simple(vec![SimpleSelector::Type {
                        namespace_prefix: None,
                        name: "div".to_string(),
                    }])),
                    Combinator::GreaterThan,
                    Box::new(Selector::Simple(vec![SimpleSelector::Type {
                        namespace_prefix: None,
                        name: "p".to_string(),
                    }]))
                ),
                Selector::Complex(
                    Box::new(Selector::Simple(vec![SimpleSelector::Type {
                        namespace_prefix: None,
                        name: "a".to_string(),
                    }])),
                    Combinator::Plus,
                    Box::new(Selector::Simple(vec![SimpleSelector::Type {
                        namespace_prefix: None,
                        name: "b".to_string(),
                    }]))
                )
            ]
        );
    }

    #[test]
    fn test_parse_selector3() {
        // h1[title="hello"] > .myclass + p, example|*

        let input = vec![
            ComponentValue::PreservedToken(Token::Ident("h1".to_string())),
            ComponentValue::SimpleBlock {
                associated_token: Token::OpenSquareBracket,
                values: vec![
                    ComponentValue::PreservedToken(Token::Ident("title".to_string())),
                    ComponentValue::PreservedToken(Token::Delim('=')),
                    ComponentValue::PreservedToken(Token::String("hello".to_string())),
                ],
            },
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Delim('>')),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Delim('.')),
            ComponentValue::PreservedToken(Token::Ident("myclass".to_string())),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Delim('+')),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Ident("p".to_string())),
            ComponentValue::PreservedToken(Token::Comma),
            ComponentValue::PreservedToken(Token::Whitespace),
            ComponentValue::PreservedToken(Token::Ident("example".to_string())),
            ComponentValue::PreservedToken(Token::Delim('|')),
            ComponentValue::PreservedToken(Token::Delim('*')),
        ];
        let mut parser = SelectorParser::new(input);
        assert_eq!(
            parser.parse_selectors_group().unwrap(),
            vec![
                Selector::Complex(
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
                    ]),),
                    Combinator::GreaterThan,
                    Box::new(Selector::Complex(
                        Box::new(Selector::Simple(vec![SimpleSelector::Class(
                            "myclass".to_string()
                        )])),
                        Combinator::Plus,
                        Box::new(Selector::Simple(vec![SimpleSelector::Type {
                            namespace_prefix: None,
                            name: "p".to_string(),
                        }]))
                    ))
                ),
                Selector::Simple(vec![SimpleSelector::Universal(Some("example".to_string()))])
            ]
        );
    }
}
