use std::cell::RefCell;
use std::iter::Peekable;
use std::ops::Deref;
use std::rc::Rc;
use std::vec::IntoIter;

use anyhow::{bail, ensure, Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::token::CssToken;
use crate::renderer::html::dom::{DomNode, NodeType};

/// - https://www.w3.org/TR/selectors-3/#simple-selectors
/// - https://www.w3.org/TR/selectors-3/#grammar
#[derive(Clone, Debug, PartialEq)]
pub enum SimpleSelector {
    Type {
        namespace_prefix: Option<String>,
        name: String,
    },
    Universal(Option<String>), // Option<namespace prefix>
    Attribute {
        namespace_prefix: Option<String>,
        name: String,
        op: Option<String>,
        value: Option<String>,
    },
    Class(String),
    Id(String),
    PseudoClass(String),
    // PseudoElement(String),
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
                        // e.g) p.class2 matches <p class="class1 class2 class3">
                        .any(|(k, v)| k == "class" && v.split(' ').any(|c| c == class_name))
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
            SimpleSelector::PseudoClass(class_name) => {
                if let NodeType::Element(elm) = &dom_node.node_type {
                    match class_name.as_str() {
                        // https://developer.mozilla.org/en-US/docs/Web/CSS/:link
                        "link" => elm.attributes.iter().any(|(k, _)| k == "href"),
                        _ => {
                            // todo
                            false
                        }
                    }
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
                        let right_node_parent = right_node.borrow().parent.as_ref()?.upgrade()?;
                        let mut ancestor = right_node_parent;

                        // Check whether the left selector exists in the ancestor of the right selector.
                        loop {
                            for simple_selector in left {
                                if simple_selector.matches(&ancestor) {
                                    return Some(Rc::clone(&ancestor));
                                }
                            }
                            if ancestor.borrow().parent.is_none() {
                                break;
                            }
                            let a = ancestor.borrow().parent.as_ref()?.upgrade()?;
                            ancestor = a;
                        }
                        None
                    }

                    // https://www.w3.org/TR/selectors-3/#child-combinators
                    Combinator::GreaterThan => {
                        // NOTE: html tag has no parent element.
                        let right_node_parent = right_node.borrow().parent.as_ref()?.upgrade()?;

                        // Check that the left selector is a parent of the right selector.
                        for simple_selector in left {
                            if simple_selector.matches(&right_node_parent) {
                                return Some(Rc::clone(&right_node_parent));
                            }
                        }
                        None
                    }

                    // https://www.w3.org/TR/selectors-3/#adjacent-sibling-combinators
                    Combinator::Plus => {
                        let mut right_node_prev_sibling =
                            right_node.borrow().prev_sibling.as_ref()?.upgrade()?;

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
                            let s = right_node_prev_sibling
                                .borrow()
                                .prev_sibling
                                .as_ref()?
                                .upgrade()?;
                            right_node_prev_sibling = s;
                        }
                    }

                    // https://www.w3.org/TR/selectors-3/#general-sibling-combinators
                    Combinator::Tilde => {
                        let mut right_node_prev_sibling =
                            right_node.borrow().prev_sibling.as_ref()?.upgrade()?;

                        loop {
                            // Non-element nodes (e.g. text between elements) are ignored when considering adjacency of elements.
                            if let NodeType::Element(_) = right_node_prev_sibling.borrow().node_type
                            {
                                for simple_selector in left {
                                    if simple_selector.matches(&right_node_prev_sibling) {
                                        return Some(right_node_prev_sibling.clone());
                                    }
                                }
                                // If not matched, continue to the next sibling.
                            }

                            // Set the previous sibling of the previous sibling if previous sibling is not Element.
                            let s = right_node_prev_sibling
                                .borrow()
                                .prev_sibling
                                .as_ref()?
                                .upgrade()?;
                            right_node_prev_sibling = s;
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
        fn calc_helper(
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
                        SimpleSelector::PseudoClass(_) => spec.1 += 1,
                    }
                }
                spec
            } else {
                let Selector::Complex(left, _, right) = current_selector else {
                    unreachable!();
                };
                let right_spec = calc_helper(right, current_specificity);
                let left_spec = calc_helper(left, current_specificity);
                (
                    left_spec.0 + right_spec.0,
                    left_spec.1 + right_spec.1,
                    left_spec.2 + right_spec.2,
                )
            }
        }

        let w = calc_helper(self, (0, 0, 0));
        w.0 * 100 + w.1 * 10 + w.2
    }
}

#[derive(Debug)]
pub struct SelectorParser {
    input: Peekable<IntoIter<ComponentValue>>,
}

impl SelectorParser {
    pub fn new(values: Vec<ComponentValue>) -> Self {
        Self {
            input: values.into_iter().peekable(),
        }
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
            match self.input.next() {
                Some(ComponentValue::PreservedToken(CssToken::Comma)) => {
                    while self
                        .input
                        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
                        .is_some()
                    {}
                    selectors.push(self.parse_selector()?);
                }
                Some(v) => {
                    bail!(
                        "Unexpected token when parsing CSS selectors in parse_selectors_group: {:?}",
                        v
                    );
                }
                None => break,
            }
        }
        Ok(selectors)
    }

    // selector
    //   : simple_selector_sequence [ combinator simple_selector_sequence ]*
    //   ;
    fn parse_selector(&mut self) -> Result<Selector> {
        let simple = Selector::Simple(self.parse_simple_selector_seq()?);

        if let Some(ComponentValue::PreservedToken(
            CssToken::Delim('+' | '>' | '~') | CssToken::Whitespace,
        )) = self.input.peek()
        {
            Ok(Selector::Complex(
                Box::new(simple),
                self.parse_combinator()?,
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
    fn parse_combinator(&mut self) -> Result<Combinator> {
        let mut is_detected_space = false;
        while self
            .input
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {
            is_detected_space = true;
        }

        if let Some(ComponentValue::PreservedToken(CssToken::Delim('+' | '>' | '~'))) =
            self.input.peek()
        {
            let Some(ComponentValue::PreservedToken(CssToken::Delim(c))) = self.input.next() else {
                unreachable!();
            };
            while self
                .input
                .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
                .is_some()
            {}
            match c {
                '+' => Ok(Combinator::Plus),
                '>' => Ok(Combinator::GreaterThan),
                '~' => Ok(Combinator::Tilde),
                _ => unreachable!(),
            }
        } else if is_detected_space {
            Ok(Combinator::Whitespace)
        } else {
            bail!(
            "Expected \"+\", \">\", \"~\", or whitespace but found {:?} when parsing CSS selectors in parse_combinator",
            self.input.peek())
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

        let v = self.input.clone().take(3).collect::<Vec<_>>();
        match (v.first(), v.get(1), v.get(2)) {
            (
                Some(ComponentValue::PreservedToken(CssToken::Ident(_) | CssToken::Delim('*'))),
                Some(ComponentValue::PreservedToken(CssToken::Delim('|'))),
                Some(ComponentValue::PreservedToken(CssToken::Ident(_))),
            )
            | (
                Some(ComponentValue::PreservedToken(CssToken::Delim('|'))),
                Some(ComponentValue::PreservedToken(CssToken::Ident(_))),
                _,
            ) => selector_seq.push(self.parse_type_selector()?),
            (
                Some(ComponentValue::PreservedToken(CssToken::Ident(_) | CssToken::Delim('*'))),
                Some(ComponentValue::PreservedToken(CssToken::Delim('|'))),
                Some(ComponentValue::PreservedToken(CssToken::Delim('*'))),
            )
            | (
                Some(ComponentValue::PreservedToken(CssToken::Delim('|'))),
                Some(ComponentValue::PreservedToken(CssToken::Delim('*'))),
                _,
            ) => selector_seq.push(self.parse_universal()?),
            (Some(ComponentValue::PreservedToken(CssToken::Ident(_))), _, _) => {
                selector_seq.push(self.parse_type_selector()?)
            }
            (Some(ComponentValue::PreservedToken(CssToken::Delim('*'))), _, _) => {
                selector_seq.push(self.parse_universal()?)
            }
            _ => {}
        }

        match self.input.peek() {
            Some(ComponentValue::PreservedToken(CssToken::Hash(..))) => {
                while let Some(ComponentValue::PreservedToken(CssToken::Hash(s, ..))) =
                    self.input.peek()
                {
                    let s = s.clone();
                    self.input.next();
                    selector_seq.push(SimpleSelector::Id(s.to_string()));
                }
            }
            Some(ComponentValue::PreservedToken(CssToken::Delim('.'))) => {
                while let Some(ComponentValue::PreservedToken(CssToken::Delim('.'))) =
                    self.input.peek()
                {
                    selector_seq.push(self.parse_class()?);
                }
            }
            Some(ComponentValue::SimpleBlock { .. }) => {
                while let Some(ComponentValue::SimpleBlock {
                    associated_token: t,
                    ..
                }) = self.input.peek()
                {
                    ensure!(
                        t == &CssToken::OpenSquareBracket,
                        "Expected \"[\" but found {:?} when parsing CSS selectors in parse_simple_selector_seq",
                        self.input.peek()
                    );
                    selector_seq.push(self.parse_attrib()?);
                }
            }
            Some(ComponentValue::PreservedToken(CssToken::Colon)) => {
                while let Some(ComponentValue::PreservedToken(CssToken::Colon)) = self.input.peek()
                {
                    selector_seq.push(self.parse_pseudo()?);
                }
            }
            _ => {}
        }

        ensure!(
            !selector_seq.is_empty(),
            "Expected type selector, universal selector, hash, class, attribute, pseudo, or negation but found {:?} when parsing CSS selectors in parse_simple_selector_seq",
            self.input.peek()
        );

        Ok(selector_seq)
    }

    // type_selector
    //   : [ namespace_prefix ]? element_name
    //   ;
    fn parse_type_selector(&mut self) -> Result<SimpleSelector> {
        let v = self.input.clone().take(2).collect::<Vec<_>>();
        match (v.first(), v.get(1)) {
            (Some(ComponentValue::PreservedToken(CssToken::Delim('|'))), _)
            | (Some(ComponentValue::PreservedToken(CssToken::Ident(_) | CssToken::Delim('*'))), Some(ComponentValue::PreservedToken(CssToken::Delim('|')))) => {
                Ok(SimpleSelector::Type {
                    namespace_prefix: Some(self.parse_namespace_prefix()?),
                    name: self.parse_element_name()?,
                })
            }
            (Some(ComponentValue::PreservedToken(CssToken::Ident(_))), _) => {
                Ok(SimpleSelector::Type {
                    namespace_prefix: None,
                    name: self.parse_element_name()?,
                })
            }
            _ => bail!(
                "Expected namespace prefix or element name but found {:?} when parsing CSS selectors in parse_type_selector",
                self.input.peek())
        }
    }

    // namespace_prefix
    //   : [ IDENT | '*' ]? '|'
    //   ;
    fn parse_namespace_prefix(&mut self) -> Result<String> {
        let v = self.input.next();
        match v.as_ref() {
            Some(ComponentValue::PreservedToken(CssToken::Delim('|'))) => {
                Ok("".to_string())
            }
            Some(ComponentValue::PreservedToken(CssToken::Ident(s))) => {
                let v = self.input.next();
                if v.as_ref() == Some(&ComponentValue::PreservedToken(CssToken::Delim('|')))
                {
                    Ok(s.clone())
                } else {
                    bail!(
                        "Expected \"|\" but found {:?} when parsing CSS selectors in parse_namespace_prefix",
                        v);
                }
            }
            Some(ComponentValue::PreservedToken(CssToken::Delim('*'))) => {
                let v = self.input.next();
                if v.as_ref() == Some(&ComponentValue::PreservedToken(CssToken::Delim('|')))
                {
                    Ok("*".to_string())
                } else {
                    bail!(
                        "Expected \"|\" but found {:?} when parsing CSS selectors in parse_namespace_prefix",
                        v);
                }
            }
            _ => bail!(
                "Expected \"|\", ident, or \"*\" but found {:?} when parsing CSS selectors in parse_namespace_prefix",
                v)
        }
    }

    // element_name
    //   : IDENT
    //   ;
    fn parse_element_name(&mut self) -> Result<String> {
        let v = self.input.next();
        if let Some(ComponentValue::PreservedToken(CssToken::Ident(s))) = v {
            Ok(s)
        } else {
            bail!(
                "Expected ident but found {:?} when parsing CSS selectors in parse_element_name",
                v
            );
        }
    }

    // universal
    //   : [ namespace_prefix ]? '*'
    //   ;
    fn parse_universal(&mut self) -> Result<SimpleSelector> {
        let v = self.input.clone().take(2).collect::<Vec<_>>();
        match (v.first(), v.get(1)) {
            (Some(ComponentValue::PreservedToken(CssToken::Delim('|'))), _)
            | (Some(ComponentValue::PreservedToken(CssToken::Ident(_) | CssToken::Delim('*'))), Some(ComponentValue::PreservedToken(CssToken::Delim('|')))) => {
                let prefix = self.parse_namespace_prefix()?;
                self.input.next();
                Ok(SimpleSelector::Universal(Some(prefix)))
            }
            (Some(ComponentValue::PreservedToken(CssToken::Delim('*'))), _) => {
                self.input.next();
                Ok(SimpleSelector::Universal(None))
            }
            _ => bail!(
                "Expected namespace prefix or \"*\" but found {:?} when parsing CSS selectors in parse_universal",
                v.first())
        }
    }

    // class
    //   : '.' IDENT
    //   ;
    fn parse_class(&mut self) -> Result<SimpleSelector> {
        let v = self.input.next();
        if let Some(ComponentValue::PreservedToken(CssToken::Delim('.'))) = v {
            Ok(SimpleSelector::Class(self.parse_element_name()?))
        } else {
            bail!(
                "Expected \".\" but found {:?} when parsing CSS selectors in parse_class",
                v
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
        let v = self.input.next();
        let mut values_in_block = if let Some(ComponentValue::SimpleBlock {
            associated_token: t,
            values: values_in_block,
        }) = v
        {
            ensure!(
                t == CssToken::OpenSquareBracket,
                "Expected \"[\" but found {:?} when parsing CSS selectors in parse_attrib",
                t
            );

            values_in_block.clone().into_iter().peekable()
        } else {
            bail!(
                "Expected simple block but found {:?} when parsing CSS selectors in parse_attrib",
                v
            );
        };

        while values_in_block
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {}

        let v = values_in_block.clone().take(2).collect::<Vec<_>>();
        let prefix = match (v.first(), v.get(1)) {
            (Some(ComponentValue::PreservedToken(CssToken::Delim('|'))), _)
            | (Some(ComponentValue::PreservedToken(CssToken::Ident(_) | CssToken::Delim('*'))), Some(ComponentValue::PreservedToken(CssToken::Delim('|')))) => {
                Some(Self::new(values_in_block.clone().collect()).parse_namespace_prefix()?)
            }
            (Some(ComponentValue::PreservedToken(CssToken::Ident(_))), _) => {
                None
            }
            _ => bail!(
                "Expected namespace prefix or ident but found {:?} when parsing CSS selectors in parse_attrib",
                v.first(),)
        };

        let v = values_in_block.next();
        let Some(ComponentValue::PreservedToken(CssToken::Ident(name))) = v else {
            bail!(
                "Expected ident but found {:?} when parsing CSS selectors in parse_attrib",
                v
            );
        };
        while values_in_block
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {}

        match values_in_block.peek() {
            Some(ComponentValue::PreservedToken(CssToken::Delim(c))) => {
                let c = *c;
                let op = if c == '=' {
                    values_in_block.next();
                    "=".to_string()
                } else if matches!(c, '^' | '$' | '*' | '~' | '|') {
                    values_in_block.next();
                    if let Some(ComponentValue::PreservedToken(CssToken::Delim('='))) =
                        values_in_block.peek()
                    {
                        values_in_block.next();
                        format!("{}=", c)
                    } else {
                        bail!(
                            "Expected \"=\" but found {:?} when parsing CSS selectors in parse_attrib",
                            values_in_block.peek()
                        )
                    }
                } else {
                    bail!(
                        "Expected \"=\", \"^=\", \"$=\", \"*=\", \"~=\", \"|=\" but found {:?} when parsing CSS selectors in parse_attrib",
                        values_in_block.peek()
                    );
                };

                while values_in_block
                    .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
                    .is_some()
                {}

                let v = values_in_block.next();
                let value = if let Some(ComponentValue::PreservedToken(
                    CssToken::Ident(s) | CssToken::String(s),
                )) = v
                {
                    Some(s)
                } else {
                    bail!("Expected ident or string but found {:?} when parsing CSS selectors in parse_attrib", v);
                };

                while values_in_block
                    .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
                    .is_some()
                {}

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
                values_in_block.peek()
            ),
        }
    }

    // pseudo
    //     : ':' ':'? [ IDENT | functional_pseudo ]
    //     ;
    fn parse_pseudo(&mut self) -> Result<SimpleSelector> {
        let v = self.input.clone().take(2).collect::<Vec<_>>();
        match (v.first(), v.get(1)) {
            (
                Some(ComponentValue::PreservedToken(CssToken::Colon)),
                Some(ComponentValue::PreservedToken(CssToken::Colon)),
            ) => {
                // pseudo-element
                self.input.next();
                self.input.next();
            }
            (Some(ComponentValue::PreservedToken(CssToken::Colon)), _) => {
                // pseudo-class
                self.input.next();
            }
            _ => bail!(
                "Expected \":\" but found {:?} when parsing CSS selectors in parse_pseudo",
                v.first()
            ),
        }

        // todo: handle functional-pseudo and pseudo-element
        let v = self.input.next();
        if let Some(ComponentValue::PreservedToken(CssToken::Ident(s))) = v {
            Ok(SimpleSelector::PseudoClass(s))
        } else {
            bail!(
                "Expected ident but found {:?} when parsing CSS selectors in parse_pseudo",
                v
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_selector_with_combinator() {
        // div > p
        let input = vec![
            ComponentValue::PreservedToken(CssToken::Ident("div".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Delim('>')),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("p".to_string())),
        ];

        assert_eq!(
            SelectorParser::new(input).parse().unwrap(),
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
    fn parse_selectors_with_comma() {
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
        assert_eq!(
            SelectorParser::new(input).parse().unwrap(),
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
    fn parse_complex_selectors() {
        // h1[title="hello"] > .myclass + p, example|*, *, *|*, *|example
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
            ComponentValue::PreservedToken(CssToken::Comma),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Delim('*')),
            ComponentValue::PreservedToken(CssToken::Comma),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Delim('*')),
            ComponentValue::PreservedToken(CssToken::Delim('|')),
            ComponentValue::PreservedToken(CssToken::Delim('*')),
            ComponentValue::PreservedToken(CssToken::Comma),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Delim('*')),
            ComponentValue::PreservedToken(CssToken::Delim('|')),
            ComponentValue::PreservedToken(CssToken::Ident("example".to_string())),
        ];
        assert_eq!(
            SelectorParser::new(input).parse().unwrap(),
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
                Selector::Simple(vec![SimpleSelector::Universal(Some("example".to_string()))]),
                Selector::Simple(vec![SimpleSelector::Universal(None)]),
                Selector::Simple(vec![SimpleSelector::Universal(Some("*".to_string()))]),
                Selector::Simple(vec![SimpleSelector::Type {
                    namespace_prefix: Some("*".to_string()),
                    name: "example".to_string(),
                }]),
            ]
        );
    }

    #[test]
    fn parse_selector_with_square_bracket() {
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
        assert_eq!(
            SelectorParser::new(input).parse().unwrap(),
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
    fn parse_selector_linked_by_dots() {
        // p.class1.class2.class3
        let input = vec![
            ComponentValue::PreservedToken(CssToken::Ident("p".to_string())),
            ComponentValue::PreservedToken(CssToken::Delim('.')),
            ComponentValue::PreservedToken(CssToken::Ident("class1".to_string())),
            ComponentValue::PreservedToken(CssToken::Delim('.')),
            ComponentValue::PreservedToken(CssToken::Ident("class2".to_string())),
            ComponentValue::PreservedToken(CssToken::Delim('.')),
            ComponentValue::PreservedToken(CssToken::Ident("class3".to_string())),
        ];
        assert_eq!(
            SelectorParser::new(input).parse().unwrap(),
            vec![Selector::Simple(vec![
                SimpleSelector::Type {
                    namespace_prefix: None,
                    name: "p".to_string(),
                },
                SimpleSelector::Class("class1".to_string()),
                SimpleSelector::Class("class2".to_string()),
                SimpleSelector::Class("class3".to_string()),
            ])]
        );
    }

    #[test]
    fn calculate_specificity() {
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
        let selector = Selector::Complex(
            Box::new(Selector::Simple(vec![SimpleSelector::Type {
                namespace_prefix: None,
                name: "UL".to_string(),
            }])),
            Combinator::Whitespace,
            Box::new(Selector::Simple(vec![SimpleSelector::Type {
                namespace_prefix: None,
                name: "LI".to_string(),
            }])),
        );
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
        let selector = Selector::Complex(
            Box::new(Selector::Simple(vec![SimpleSelector::Type {
                namespace_prefix: None,
                name: "UL".to_string(),
            }])),
            Combinator::Whitespace,
            Box::new(Selector::Complex(
                Box::new(Selector::Simple(vec![SimpleSelector::Type {
                    namespace_prefix: None,
                    name: "OL".to_string(),
                }])),
                Combinator::Whitespace,
                Box::new(Selector::Simple(vec![
                    SimpleSelector::Type {
                        namespace_prefix: None,
                        name: "LI".to_string(),
                    },
                    SimpleSelector::Class("red".to_string()),
                ])),
            )),
        );
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

        // todo: handle functional pseudo-class
        // #s12:not(FOO)
        // let selector = Selector::Simple(vec![
        //     SimpleSelector::Id("s12".to_string()),
        // ]);
        // assert_eq!(selector.calc_specificity(), 101);
    }
}
