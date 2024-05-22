use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::Deref;
use std::rc::Rc;

use anyhow::{bail, Ok, Result};

use crate::css::cssom::ComponentValue;
use crate::css::tokenizer::CssToken;
use crate::html::dom::{DomNode, NodeType};

/// - https://www.w3.org/TR/selectors-3/#simple-selectors
/// - https://www.w3.org/TR/selectors-3/#grammar
#[derive(Clone, Debug, PartialEq)]
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
    // PseudoClass,
}

impl SimpleSelector {
    pub fn matches(&self, dom_node: &Rc<RefCell<DomNode>>) -> bool {
        let dom_node = dom_node.borrow();

        match self {
            SimpleSelector::Type {
                namespace_prefix,
                name,
            } => {
                if namespace_prefix.is_some() {
                    unimplemented!();
                }

                if let NodeType::Element(elm) = &dom_node.node_type {
                    elm.tag_name == *name
                } else {
                    false
                }
            }
            SimpleSelector::Class(class_name) => {
                if let NodeType::Element(elm) = &dom_node.node_type {
                    elm.attributes
                        .iter()
                        .any(|(k, v)| k == "class" && v == class_name)
                } else {
                    false
                }
            }
            SimpleSelector::Id(id) => {
                if let NodeType::Element(elm) = &dom_node.node_type {
                    elm.attributes.iter().any(|(k, v)| k == "id" && v == id)
                } else {
                    false
                }
            }
            _ => unimplemented!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Combinator {
    Whitespace,
    GreaterThan,
    Plus,
    Tilde,
}

/// https://www.w3.org/TR/selectors-3/#selector-syntax
#[derive(Clone, Debug, PartialEq)]
pub enum Selector {
    Simple(Vec<SimpleSelector>),

    // The left value can have a Complex selector in this data structure,
    // but it must be a Simple selector because of the right associativity of the selector.
    // https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_selectors/Selector_structure#complex_selector
    Complex(Box<Selector>, Combinator, Box<Selector>),
}

impl Selector {
    pub fn matches(&self, dom_node: &Rc<RefCell<DomNode>>) -> bool {
        /// Returns the DOM node that the selector constructed in the current tree evaluates for the node backtracked from the target node.
        /// If the selector does not match the node, the whole selector tree does not match the node, so this function returns None.
        fn matches_helper(
            current_selector: &Selector,
            target_dom_node: &Rc<RefCell<DomNode>>,
        ) -> Option<Rc<RefCell<DomNode>>> {
            if let Selector::Simple(selectors) = current_selector {
                // Simple, base-case
                let success_match = selectors
                    .iter()
                    .all(|simple_selector| simple_selector.matches(target_dom_node));
                if success_match {
                    Some(Rc::clone(target_dom_node))
                } else {
                    None
                }
            } else {
                // Complex
                let (left, combinator, right) =
                    if let Selector::Complex(left, combinator, right) = current_selector {
                        let Selector::Simple(left) = left.deref() else {
                            unreachable!();
                        };
                        (left, combinator, right)
                    } else {
                        unreachable!();
                    };

                let right_node = matches_helper(right, target_dom_node)?;

                // https://developer.mozilla.org/en-US/docs/Learn/CSS/Building_blocks/Selectors/Combinators
                match combinator {
                    // https://www.w3.org/TR/selectors-3/#descendant-combinators
                    Combinator::Whitespace => {
                        // NOTE: html tag has no parent element.
                        let right_node_parent =
                            Rc::clone(right_node.borrow().parent_node.as_ref()?);

                        // Check whether the left selector exists in the ancestor of the right selector.
                        let mut ancestor = right_node_parent;
                        loop {
                            for simple_selector in left {
                                if simple_selector.matches(&ancestor) {
                                    return Some(ancestor);
                                }
                            }
                            if ancestor.borrow().parent_node.as_ref().is_none() {
                                break;
                            }
                            let parent = Rc::clone(ancestor.borrow().parent_node.as_ref()?);
                            ancestor = Rc::clone(&parent);
                        }
                        None
                    }

                    // https://www.w3.org/TR/selectors-3/#child-combinators
                    Combinator::GreaterThan => {
                        // NOTE: html tag has no parent element.
                        let right_node_parent =
                            Rc::clone(right_node.borrow().parent_node.as_ref()?);

                        // Check that the left selector is a parent of the right selector.
                        for simple_selector in left {
                            if simple_selector.matches(&right_node_parent) {
                                return Some(right_node_parent);
                            }
                        }
                        None
                    }

                    // https://www.w3.org/TR/selectors-3/#adjacent-sibling-combinators
                    Combinator::Plus => {
                        let mut right_node_prev_sibling =
                            Rc::clone(right_node.as_ref().borrow().previous_sibling.as_ref()?);

                        loop {
                            // Non-element nodes (e.g. text between elements) are ignored when considering adjacency of elements.
                            if let NodeType::Element(_) = right_node_prev_sibling.borrow().node_type
                            {
                                for simple_selector in left {
                                    if simple_selector.matches(&right_node_prev_sibling) {
                                        return Some(Rc::clone(&right_node_prev_sibling));
                                    }
                                }
                                return None;
                            }

                            // Set the previous sibling of the previous sibling if previous sibling is not Element.
                            let s = Rc::clone(
                                right_node_prev_sibling
                                    .as_ref()
                                    .borrow()
                                    .previous_sibling
                                    .as_ref()?,
                            );
                            right_node_prev_sibling = Rc::clone(&s);
                        }
                    }

                    // https://www.w3.org/TR/selectors-3/#general-sibling-combinators
                    Combinator::Tilde => {
                        let mut right_node_prev_sibling =
                            Rc::clone(right_node.as_ref().borrow().previous_sibling.as_ref()?);

                        loop {
                            // Non-element nodes (e.g. text between elements) are ignored when considering adjacency of elements.
                            if let NodeType::Element(_) = right_node_prev_sibling.borrow().node_type
                            {
                                for simple_selector in left {
                                    if simple_selector.matches(&right_node_prev_sibling) {
                                        return Some(Rc::clone(&right_node_prev_sibling));
                                    }
                                }
                                // If not matched, continue to the next sibling.
                            }

                            // Set the previous sibling of the previous sibling if previous sibling is not Element.
                            let s = Rc::clone(
                                right_node_prev_sibling
                                    .as_ref()
                                    .borrow()
                                    .previous_sibling
                                    .as_ref()?,
                            );
                            right_node_prev_sibling = Rc::clone(&s);
                        }
                    }
                }
            }
        }

        matches_helper(self, dom_node).is_some()
    }

    /// - https://www.w3.org/TR/selectors-3/#specificity
    /// - https://developer.mozilla.org/en-US/docs/Web/CSS/Specificity
    pub fn calc_specificity(&self) -> u32 {
        /// Returns `(a, b, c)`, where `a` is the number of ID selectors, `b` is the number of class selectors, attributes selectors, and pseudo-classes,
        /// and `c` is the number of type selectors and pseudo-elements.
        fn helper(
            current_selector: &Selector,
            current_specificity: (u32, u32, u32),
        ) -> (u32, u32, u32) {
            if let Selector::Simple(selectors) = current_selector {
                let mut spec = current_specificity;
                for simple_selector in selectors {
                    match simple_selector {
                        SimpleSelector::Type { .. } => spec.2 += 1,
                        SimpleSelector::Universal(_) => {} // Ignore the universal selector.
                        SimpleSelector::Attribute { .. } => spec.1 += 1,
                        SimpleSelector::Class(_) => spec.1 += 1,
                        SimpleSelector::Id(_) => spec.0 += 1,
                        // todo: PseudoClass
                    }
                }
                spec
            } else {
                let Selector::Complex(left, _, right) = current_selector else {
                    unreachable!();
                };
                let right_spec = helper(right, current_specificity);
                let left_spec = helper(left, current_specificity);
                (
                    left_spec.0 + right_spec.0,
                    left_spec.1 + right_spec.1,
                    left_spec.2 + right_spec.2,
                )
            }
        }

        let w = helper(self, (0, 0, 0));
        w.0 * 100 + w.1 * 10 + w.2
    }
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
                Some(ComponentValue::PreservedToken(CssToken::Comma)) => {
                    while let Some(ComponentValue::PreservedToken(CssToken::Whitespace)) =
                        self.input.front()
                    {
                        self.input.pop_front();
                    }
                    selectors.push(self.parse_selector()?);
                }
                None => break,
                t => {
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
        while let Some(ComponentValue::PreservedToken(CssToken::Whitespace)) = self.input.front() {
            self.input.pop_front();
            is_detected_space = true;
        }

        match self.input.front() {
            Some(ComponentValue::PreservedToken(CssToken::Delim('+'))) => {
                self.input.pop_front();
                while let Some(ComponentValue::PreservedToken(CssToken::Whitespace)) =
                    self.input.front()
                {
                    self.input.pop_front();
                }
                Some(Combinator::Plus)
            }
            Some(ComponentValue::PreservedToken(CssToken::Delim('>'))) => {
                self.input.pop_front();
                while let Some(ComponentValue::PreservedToken(CssToken::Whitespace)) =
                    self.input.front()
                {
                    self.input.pop_front();
                }
                Some(Combinator::GreaterThan)
            }
            Some(ComponentValue::PreservedToken(CssToken::Delim('~'))) => {
                self.input.pop_front();
                while let Some(ComponentValue::PreservedToken(CssToken::Whitespace)) =
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
            Some(ComponentValue::PreservedToken(CssToken::Hash(..))) => {
                while let Some(ComponentValue::PreservedToken(CssToken::Hash(s, ..))) =
                    self.input.front()
                {
                    let s = s.clone();
                    self.input.pop_front();
                    selector_seq.push(SimpleSelector::Id(s.to_string()));
                }
            }
            Some(ComponentValue::PreservedToken(CssToken::Delim('.'))) => {
                while let Some(ComponentValue::PreservedToken(CssToken::Delim('.'))) =
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
                    if t != &CssToken::OpenSquareBracket {
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
            if self.input.front() == Some(&ComponentValue::PreservedToken(CssToken::Delim('*'))) {
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
                Some(ComponentValue::PreservedToken(CssToken::Hash(..))) => {
                    while let Some(ComponentValue::PreservedToken(CssToken::Hash(s, ..))) =
                        self.input.front()
                    {
                        let s = s.clone();
                        self.input.pop_front();
                        selector_seq.push(SimpleSelector::Id(s.to_string()));
                    }
                }
                Some(ComponentValue::PreservedToken(CssToken::Delim('.'))) => {
                    while let Some(ComponentValue::PreservedToken(CssToken::Delim('.'))) =
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
                        if t != &CssToken::OpenSquareBracket {
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
    #[allow(dead_code)]
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
            Some(ComponentValue::PreservedToken(CssToken::Delim('|'))) => {
                self.input.pop_front();
                Some("".to_string())
            }
            Some(ComponentValue::PreservedToken(CssToken::Ident(s))) => {
                if self.input.get(1) == Some(&ComponentValue::PreservedToken(CssToken::Delim('|')))
                {
                    let s = s.clone();
                    self.input.pop_front();
                    self.input.pop_front();
                    Some(s)
                } else {
                    None
                }
            }
            Some(ComponentValue::PreservedToken(CssToken::Delim('*'))) => {
                if self.input.get(1) == Some(&ComponentValue::PreservedToken(CssToken::Delim('|')))
                {
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
        let comp = self.input.pop_front();
        if let Some(ComponentValue::PreservedToken(CssToken::Ident(s))) = comp {
            Ok(s)
        } else {
            bail!(
                "Expected ident but found {:?} when parsing CSS selectors in parse_element_name",
                comp
            );
        }
    }

    // universal
    //   : [ namespace_prefix ]? '*'
    //   ;
    #[allow(dead_code)]
    fn parse_universal(&mut self) -> Result<SimpleSelector> {
        let prefix = self.parse_namespace_prefix();
        let comp = self.input.pop_front();
        if comp == Some(ComponentValue::PreservedToken(CssToken::Delim('*'))) {
            Ok(SimpleSelector::Universal(prefix))
        } else {
            bail!(
                "Expected \"*\" but found {:?} when parsing CSS selectors in parse_universal",
                comp
            );
        }
    }

    // class
    //   : '.' IDENT
    //   ;
    fn parse_class(&mut self) -> Result<SimpleSelector> {
        let comp = self.input.pop_front();
        if comp == Some(ComponentValue::PreservedToken(CssToken::Delim('.'))) {
            Ok(SimpleSelector::Class(self.parse_element_name()?))
        } else {
            bail!(
                "Expected \".\" but found {:?} when parsing CSS selectors in parse_class",
                comp
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
        let comp = self.input.pop_front();
        if let Some(ComponentValue::SimpleBlock {
            associated_token: t,
            values,
        }) = comp.clone()
        {
            if t != CssToken::OpenSquareBracket {
                bail!(
                    "Expected \"[\" but found {:?} when parsing CSS selectors in parse_attrib",
                    comp
                );
            }

            let mut values: VecDeque<ComponentValue> = VecDeque::from(values);

            while let Some(ComponentValue::PreservedToken(CssToken::Whitespace)) = values.front() {
                values.pop_front();
            }

            // Parse prefix
            let prefix = match self.input.front() {
                Some(ComponentValue::PreservedToken(CssToken::Delim('|'))) => {
                    self.input.pop_front();
                    Some("".to_string())
                }
                Some(ComponentValue::PreservedToken(CssToken::Ident(s))) => {
                    if self.input.get(1)
                        == Some(&ComponentValue::PreservedToken(CssToken::Delim('|')))
                    {
                        let s = s.clone();
                        self.input.pop_front();
                        self.input.pop_front();
                        Some(s)
                    } else {
                        None
                    }
                }
                Some(ComponentValue::PreservedToken(CssToken::Delim('*'))) => {
                    if self.input.get(1)
                        == Some(&ComponentValue::PreservedToken(CssToken::Delim('|')))
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

            let comp = values.pop_front();
            let Some(ComponentValue::PreservedToken(CssToken::Ident(name))) = comp else {
                bail!(
                    "Expected ident but found {:?} when parsing CSS selectors in parse_attrib",
                    comp
                );
            };
            while let Some(ComponentValue::PreservedToken(CssToken::Whitespace)) = values.front() {
                values.pop_front();
            }

            match values.front() {
                Some(ComponentValue::PreservedToken(CssToken::Delim(c))) => {
                    let c = *c;
                    let op = if c == '=' {
                        values.pop_front();
                        "=".to_string()
                    } else if let '^' | '$' | '*' | '~' | '|' = c {
                        values.pop_front();
                        if let Some(ComponentValue::PreservedToken(CssToken::Delim('='))) =
                            values.front()
                        {
                            values.pop_front();
                            format!("{}=", c)
                        } else {
                            bail!(
                                "Expected \"=\" but found {:?} when parsing CSS selectors in parse_attrib",
                                values.front())
                        }
                    } else {
                        bail!(
                            "Expected \"=\", \"^=\", \"$=\", \"*=\", \"~=\", \"|=\" but found {:?} when parsing CSS selectors in parse_attrib",
                            values.front()
                        );
                    };

                    while let Some(ComponentValue::PreservedToken(CssToken::Whitespace)) =
                        values.front()
                    {
                        values.pop_front();
                    }

                    let comp = values.pop_front();
                    let value = if let Some(ComponentValue::PreservedToken(CssToken::Ident(s)))
                    | Some(ComponentValue::PreservedToken(CssToken::String(s))) =
                        comp
                    {
                        Some(s)
                    } else {
                        bail!("Expected ident or string but found {:?} when parsing CSS selectors in parse_attrib", comp);
                    };

                    while let Some(ComponentValue::PreservedToken(CssToken::Whitespace)) =
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
                None => Ok(SimpleSelector::Attribute {
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
                comp
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
            ComponentValue::PreservedToken(CssToken::Ident("div".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Delim('>')),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("p".to_string())),
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
            ComponentValue::PreservedToken(CssToken::Ident("div".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Delim('>')),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("p".to_string())),
            ComponentValue::PreservedToken(CssToken::Comma),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("a".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Delim('+')),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("b".to_string())),
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
            ComponentValue::PreservedToken(CssToken::Ident("h1".to_string())),
            ComponentValue::SimpleBlock {
                associated_token: CssToken::OpenSquareBracket,
                values: vec![
                    ComponentValue::PreservedToken(CssToken::Ident("title".to_string())),
                    ComponentValue::PreservedToken(CssToken::Delim('=')),
                    ComponentValue::PreservedToken(CssToken::String("hello".to_string())),
                ],
            },
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Delim('>')),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Delim('.')),
            ComponentValue::PreservedToken(CssToken::Ident("myclass".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Delim('+')),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("p".to_string())),
            ComponentValue::PreservedToken(CssToken::Comma),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("example".to_string())),
            ComponentValue::PreservedToken(CssToken::Delim('|')),
            ComponentValue::PreservedToken(CssToken::Delim('*')),
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

    #[test]
    fn test_parse_selector4() {
        // a[href^="https"][href$=".org"]

        let input = vec![
            ComponentValue::PreservedToken(CssToken::Ident("a".to_string())),
            ComponentValue::SimpleBlock {
                associated_token: CssToken::OpenSquareBracket,
                values: vec![
                    ComponentValue::PreservedToken(CssToken::Ident("href".to_string())),
                    ComponentValue::PreservedToken(CssToken::Delim('^')),
                    ComponentValue::PreservedToken(CssToken::Delim('=')),
                    ComponentValue::PreservedToken(CssToken::String("https".to_string())),
                ],
            },
            ComponentValue::SimpleBlock {
                associated_token: CssToken::OpenSquareBracket,
                values: vec![
                    ComponentValue::PreservedToken(CssToken::Ident("href".to_string())),
                    ComponentValue::PreservedToken(CssToken::Delim('$')),
                    ComponentValue::PreservedToken(CssToken::Delim('=')),
                    ComponentValue::PreservedToken(CssToken::String(".org".to_string())),
                ],
            },
        ];
        let mut parser = SelectorParser::new(input);
        assert_eq!(
            parser.parse_selectors_group().unwrap(),
            vec![Selector::Simple(vec![
                SimpleSelector::Type {
                    namespace_prefix: None,
                    name: "a".to_string(),
                },
                SimpleSelector::Attribute {
                    namespace_prefix: None,
                    name: "href".to_string(),
                    op: Some("^=".to_string()),
                    value: Some("https".to_string()),
                },
                SimpleSelector::Attribute {
                    namespace_prefix: None,
                    name: "href".to_string(),
                    op: Some("$=".to_string()),
                    value: Some(".org".to_string()),
                },
            ])]
        );
    }

    #[test]
    fn test_calc_specificity() {
        // *
        let selector = Selector::Simple(vec![SimpleSelector::Universal(None)]);
        assert_eq!(selector.calc_specificity(), 0);

        // LI
        let selector = Selector::Simple(vec![SimpleSelector::Type {
            namespace_prefix: None,
            name: "LI".to_string(),
        }]);
        assert_eq!(selector.calc_specificity(), 1);

        // UL LI
        let selector = Selector::Simple(vec![
            SimpleSelector::Type {
                namespace_prefix: None,
                name: "UL".to_string(),
            },
            SimpleSelector::Type {
                namespace_prefix: None,
                name: "LI".to_string(),
            },
        ]);
        assert_eq!(selector.calc_specificity(), 2);

        // UL OL + LI
        let selector = Selector::Complex(
            Box::new(Selector::Simple(vec![SimpleSelector::Type {
                namespace_prefix: None,
                name: "UL".to_string(),
            }])),
            Combinator::Whitespace,
            Box::new(Selector::Complex(
                Box::new(Selector::Simple(vec![SimpleSelector::Type {
                    namespace_prefix: None,
                    name: "LI".to_string(),
                }])),
                Combinator::Plus,
                Box::new(Selector::Simple(vec![SimpleSelector::Type {
                    namespace_prefix: None,
                    name: "LI".to_string(),
                }])),
            )),
        );
        assert_eq!(selector.calc_specificity(), 3);

        // H1 + *[REL=up]
        let selector = Selector::Complex(
            Box::new(Selector::Simple(vec![SimpleSelector::Type {
                namespace_prefix: None,
                name: "H1".to_string(),
            }])),
            Combinator::Plus,
            Box::new(Selector::Simple(vec![
                SimpleSelector::Universal(None),
                SimpleSelector::Attribute {
                    namespace_prefix: None,
                    name: "REL".to_string(),
                    op: Some("=".to_string()),
                    value: Some("up".to_string()),
                },
            ])),
        );
        assert_eq!(selector.calc_specificity(), 11);

        // UL OL LI.red
        let selector = Selector::Simple(vec![
            SimpleSelector::Type {
                namespace_prefix: None,
                name: "UL".to_string(),
            },
            SimpleSelector::Type {
                namespace_prefix: None,
                name: "OL".to_string(),
            },
            SimpleSelector::Type {
                namespace_prefix: None,
                name: "LI".to_string(),
            },
            SimpleSelector::Class("red".to_string()),
        ]);
        assert_eq!(selector.calc_specificity(), 13);

        // LI.red.level
        let selector = Selector::Simple(vec![
            SimpleSelector::Type {
                namespace_prefix: None,
                name: "LI".to_string(),
            },
            SimpleSelector::Class("red".to_string()),
            SimpleSelector::Class("level".to_string()),
        ]);
        assert_eq!(selector.calc_specificity(), 21);

        // #x34y
        let selector = Selector::Simple(vec![SimpleSelector::Id("x34y".to_string())]);
        assert_eq!(selector.calc_specificity(), 100);

        // todo: Implement pseudo-class
        // #s12:not(FOO)
        // let selector = Selector::Simple(vec![SimpleSelector::Id("s12".to_string())]);
        // assert_eq!(selector.calc_specificity(), 101);
    }
}
