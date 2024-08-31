use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{bail, Ok, Result};
use thiserror::Error;

use crate::css::cssom::StyleSheet;
use crate::css::parser::CssParser;
use crate::css::tokenizer::CssTokenizer;
use crate::html::dom::{DocumentTree, DomNode, Element, NodeType};
use crate::html::tokenizer::{HtmlToken, HtmlTokenizer, TokenizationState};

#[derive(Error, Debug)]
#[error("{message} (in the HTML tree construction stage)\nCurrent HTML token: {current_token:?}\nCurrent DOM tree:\n{current_tree}")]
struct ParseError {
    message: String,
    current_token: HtmlToken,
    current_tree: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    InFrameset,
    Text,
    AfterBody,
    AfterAfterBody,
}

/// https://html.spec.whatwg.org/multipage/parsing.html#overview-of-the-parsing-model
#[derive(Debug)]
pub struct HtmlParser {
    insertion_mode: InsertionMode,
    tokenizer: HtmlTokenizer,
    stack: Vec<Rc<RefCell<DomNode>>>,

    // When the insertion mode is switched to "text" or "in table text", the original insertion mode is also set.
    // This is the insertion mode to which the tree construction stage will return.
    orig_insertion_mode: Option<InsertionMode>,
}

impl HtmlParser {
    pub fn new(tokenizer: HtmlTokenizer) -> Self {
        Self {
            insertion_mode: InsertionMode::Initial,
            tokenizer,
            stack: Vec::new(),
            orig_insertion_mode: None,
        }
    }

    /// Returns a Document object node and its associated list of CSS style sheets.
    pub fn parse(&mut self) -> Result<(Rc<RefCell<DomNode>>, Vec<StyleSheet>)> {
        // The output of the whole parsing (tree construction) is a Document object.
        let document_node = Rc::new(RefCell::new(DomNode::new(NodeType::Document)));

        // The document has an associated list of zero or more CSS style sheets.
        // This is an ordered list that contains:
        // 1. Any CSS style sheets created from HTTP Link headers, in header order
        // 2. Any CSS style sheets associated with the DocumentOrShadowRoot, in tree order
        // https://drafts.csswg.org/cssom/#documentorshadowroot-document-or-shadow-root-css-style-sheets
        let mut style_sheets = Vec::new();

        let mut end_of_parsing = false;
        while !end_of_parsing {
            let token = self.tokenizer.consume_token();

            loop {
                // https://html.spec.whatwg.org/multipage/parsing.html#tree-construction
                match &self.insertion_mode {
                    // https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
                    InsertionMode::Initial => {
                        match &token {
                            HtmlToken::Character(c) if Self::is_blank(*c) => {}
                            HtmlToken::Comment(_) => {
                                unimplemented!("token: {:?}", token);
                            }
                            HtmlToken::Doctype {
                                name,
                                public_identifier,
                                system_identifier,
                                ..
                            } => {
                                if (name.is_some() && name.clone().unwrap() != "html")
                                    || public_identifier.is_some()
                                    || (system_identifier.is_some()
                                        && system_identifier.clone().unwrap()
                                            != "about:legacy-compat")
                                {
                                    bail!(ParseError {
                                        message: "Invalid doctype".to_string(),
                                        current_token: token,
                                        current_tree: DocumentTree::build(Rc::clone(
                                            &document_node
                                        ))?
                                        .to_string(),
                                    });
                                }
                                DomNode::append_child(
                                    &document_node,
                                    DomNode::new(NodeType::DocumentType(match name {
                                        Some(name) => name.clone(),
                                        None => String::new(),
                                    })),
                                );
                                self.insertion_mode = InsertionMode::BeforeHtml;
                            }
                            _ => {
                                self.insertion_mode = InsertionMode::BeforeHtml;
                                continue; // reprocess the token
                            }
                        }
                    }

                    // https://html.spec.whatwg.org/multipage/parsing.html#the-before-html-insertion-mode
                    InsertionMode::BeforeHtml => match &token {
                        HtmlToken::Doctype { .. } => {
                            eprintln!("parse error, ignored the token: {:?}", token);
                        }
                        HtmlToken::Character(c) if Self::is_blank(*c) => {}
                        HtmlToken::StartTag {
                            tag_name,
                            attributes,
                            ..
                        } if tag_name == "html" => {
                            let n = DomNode::append_child(
                                &document_node,
                                DomNode::new(NodeType::Element(Element {
                                    tag_name: tag_name.clone(),
                                    attributes: attributes.clone(),
                                })),
                            );
                            self.stack.push(Rc::clone(&n));
                            self.insertion_mode = InsertionMode::BeforeHead;
                        }
                        HtmlToken::EndTag { tag_name, .. } => {
                            if let "head" | "body" | "html" | "br" = tag_name.as_str() {
                                let n = DomNode::append_child(
                                    &document_node,
                                    DomNode::new(NodeType::Element(Element {
                                        tag_name: "html".to_string(),
                                        attributes: Vec::new(),
                                    })),
                                );
                                self.stack.push(Rc::clone(&n));
                                self.insertion_mode = InsertionMode::BeforeHead;
                            } else {
                                eprintln!("parse error, ignored the token: {:?}", token);
                            }
                        }
                        _ => {
                            let n = DomNode::append_child(
                                &document_node,
                                DomNode::new(NodeType::Element(Element {
                                    tag_name: "html".to_string(),
                                    attributes: Vec::new(),
                                })),
                            );
                            self.stack.push(Rc::clone(&n));
                            self.insertion_mode = InsertionMode::BeforeHead;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#the-before-head-insertion-mode
                    InsertionMode::BeforeHead => match &token {
                        HtmlToken::Character(c) if Self::is_blank(*c) => {}
                        HtmlToken::Doctype { .. } => {
                            eprintln!("parse error, ignored the token: {:?}", token);
                        }
                        HtmlToken::StartTag {
                            tag_name,
                            attributes,
                            ..
                        } => match tag_name.as_str() {
                            "head" => {
                                self.insert_element(tag_name, attributes);
                                self.insertion_mode = InsertionMode::InHead;
                            }
                            _ => unimplemented!("token: {:?}", token),
                        },
                        HtmlToken::EndTag { tag_name, .. } => {
                            if let "head" | "body" | "html" | "br" = tag_name.as_str() {
                                self.insert_element(tag_name, &Vec::new());
                                self.insertion_mode = InsertionMode::InHead;
                                continue;
                            } else {
                                eprintln!("parse error, ignored the token: {:?}", token);
                            }
                        }
                        _ => {
                            self.insert_element("head", &Vec::new());
                            self.insertion_mode = InsertionMode::InHead;
                            continue;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
                    InsertionMode::InHead => {
                        match &token {
                            HtmlToken::Character(c) if Self::is_blank(*c) => {
                                self.insert_char_to_token(*c);
                            }
                            HtmlToken::Comment(comment) => {
                                self.insert_comment(comment.clone());
                            }
                            HtmlToken::Doctype { .. } => {
                                eprintln!("parse error, ignored the token: {:?}", token);
                            }
                            HtmlToken::StartTag {
                                tag_name,
                                attributes,
                                ..
                            } => match tag_name.as_str() {
                                "meta" => {
                                    DomNode::append_child(
                                        self.stack.last().unwrap(),
                                        DomNode::new(NodeType::Element(Element {
                                            tag_name: tag_name.clone(),
                                            attributes: attributes.clone(),
                                        })),
                                    );
                                    // Insert the element (including pushing it onto the stack of open elements) and pop it immediately,
                                    // so the content of the stack is not changed here.
                                }
                                "title" => {
                                    // Quite simplified
                                    self.insert_element(tag_name, attributes);
                                    self.orig_insertion_mode = Some(InsertionMode::InHead);
                                    self.insertion_mode = InsertionMode::Text;
                                }
                                "style" => {
                                    // https://html.spec.whatwg.org/multipage/parsing.html#generic-raw-text-element-parsing-algorithm
                                    self.insert_element(tag_name, attributes);
                                    self.tokenizer.change_state(TokenizationState::RawText);
                                    self.orig_insertion_mode = Some(InsertionMode::InHead);
                                    self.insertion_mode = InsertionMode::Text;
                                }
                                _ => unimplemented!("token: {:?}", token),
                            },
                            HtmlToken::EndTag { tag_name, .. } if tag_name == "head" => {
                                let elm = self.stack.pop().unwrap();
                                if let NodeType::Element(elm) = &elm.borrow().node_type {
                                    if elm.tag_name != "head" {
                                        bail!(ParseError {
                                            message: "Expected head element".to_string(),
                                            current_token: token,
                                            current_tree: DocumentTree::build(Rc::clone(
                                                &document_node
                                            ))?
                                            .to_string(),
                                        });
                                    }
                                } else {
                                    bail!(ParseError {
                                        message: "Expected head element".to_string(),
                                        current_token: token,
                                        current_tree: DocumentTree::build(Rc::clone(
                                            &document_node
                                        ))?
                                        .to_string(),
                                    });
                                }
                                self.insertion_mode = InsertionMode::AfterHead;
                            }
                            _ => {
                                unimplemented!("token: {:?}", token);
                            }
                        }
                    }

                    // https://html.spec.whatwg.org/multipage/parsing.html#the-after-head-insertion-mode
                    InsertionMode::AfterHead => match &token {
                        HtmlToken::Character(c) if Self::is_blank(*c) => {
                            self.insert_char_to_token(*c);
                        }
                        HtmlToken::Doctype { .. } => {
                            eprintln!("parse error, ignored the token: {:?}", token);
                        }
                        HtmlToken::StartTag {
                            tag_name,
                            attributes,
                            ..
                        } => match tag_name.as_str() {
                            "body" => {
                                self.insert_element(tag_name, attributes);
                                self.insertion_mode = InsertionMode::InBody;
                            }
                            "frameset" => {
                                self.insert_element(tag_name, attributes);
                                self.insertion_mode = InsertionMode::InFrameset;
                            }
                            _ => unimplemented!("token: {:?}", token),
                        },
                        _ => {
                            unimplemented!("token: {:?}", token);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inbody
                    InsertionMode::InBody => match &token {
                        HtmlToken::Character(c) => match c {
                            '\u{0000}' => eprintln!("parse error, ignored the token: {:?}", token),
                            _ => {
                                self.insert_char_to_token(*c);
                            }
                        },
                        HtmlToken::Comment(comment) => {
                            self.insert_comment(comment.clone());
                        }
                        HtmlToken::Doctype { .. } => {
                            eprintln!("parse error, ignored the token: {:?}", token)
                        }
                        HtmlToken::StartTag {
                            tag_name,
                            attributes,
                            ..
                        } => match tag_name.as_str() {
                            "a" | "div" | "p" | "ul" => {
                                self.insert_element(tag_name, attributes);
                            }
                            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                                let mut need_to_pop = false;
                                if let Some(n) = &self.stack.last() {
                                    if let NodeType::Element(elm) = &n.borrow().node_type {
                                        if let "h1" | "h2" | "h3" | "h4" | "h5" | "h6" =
                                            elm.tag_name.as_str()
                                        {
                                            eprintln!("parse error");
                                            need_to_pop = true;
                                        }
                                    }
                                }
                                if need_to_pop {
                                    self.stack.pop();
                                }
                                self.insert_element(tag_name, attributes);
                            }
                            "li" => {
                                let mut node_type = self.get_current_elm_name().unwrap();

                                loop {
                                    if node_type.as_str() == "li" {
                                        self.generate_implied_end_tags(Some("li"));
                                        if self.get_current_elm_name().unwrap().as_str() != "li" {
                                            eprintln!("parse error");
                                        }
                                        loop {
                                            if let Some(n) = &self.stack.pop() {
                                                if let NodeType::Element(elm) =
                                                    &n.borrow().node_type
                                                {
                                                    if elm.tag_name == "li" {
                                                        break;
                                                    }
                                                }
                                            } else {
                                                bail!(ParseError {
                                                    message: "li element not found".to_string(),
                                                    current_token: token,
                                                    current_tree: DocumentTree::build(Rc::clone(
                                                        &document_node
                                                    ))?
                                                    .to_string(),
                                                });
                                            }
                                        }
                                        break;
                                    }

                                    // This is quite simplified.
                                    if let "address" | "div" | "p" = node_type.as_str() {
                                        // Set the node to the previous entry in the stack of open elements.
                                        node_type = if let Some(node) =
                                            &self.stack.get(self.stack.len() - 2)
                                        {
                                            let NodeType::Element(elm) = &node.borrow().node_type
                                            else {
                                                bail!(ParseError {
                                                    message: "The node is not an element"
                                                        .to_string(),
                                                    current_token: token,
                                                    current_tree: DocumentTree::build(Rc::clone(
                                                        &document_node
                                                    ))?
                                                    .to_string(),
                                                });
                                            };
                                            elm.tag_name.clone()
                                        } else {
                                            bail!(ParseError {
                                                message: "Previous node not found".to_string(),
                                                current_token: token,
                                                current_tree: DocumentTree::build(Rc::clone(
                                                    &document_node
                                                ))?
                                                .to_string(),
                                            });
                                        };
                                        continue;
                                    } else {
                                        // If node is in the special category, but is not an address, div, or p element
                                        break;
                                    }
                                }
                                self.insert_element(tag_name, attributes);
                            }
                            _ => {
                                unimplemented!("token: {:?}", token);
                            }
                        },
                        HtmlToken::EndTag { tag_name, .. } => match tag_name.as_str() {
                            "a" => {
                                if self.get_current_elm_name().unwrap().as_str() != "a" {
                                    eprintln!("parse error");
                                }
                                loop {
                                    if let Some(n) = &self.stack.pop() {
                                        if let NodeType::Element(elm) = &n.borrow().node_type {
                                            if elm.tag_name == "a" {
                                                break;
                                            }
                                        }
                                    } else {
                                        bail!(ParseError {
                                            message: "a element not found".to_string(),
                                            current_token: token,
                                            current_tree: DocumentTree::build(Rc::clone(
                                                &document_node
                                            ))?
                                            .to_string(),
                                        });
                                    }
                                }
                            }
                            "body" => {
                                self.insertion_mode = InsertionMode::AfterBody;
                            }
                            "div" | "ul" => {
                                self.generate_implied_end_tags(None);
                                if self.get_current_elm_name().unwrap().as_str() != tag_name {
                                    eprintln!("parse error");
                                }
                                while let Some(n) = self.stack.pop() {
                                    if let NodeType::Element(elm) = &n.borrow().node_type {
                                        if elm.tag_name == *tag_name {
                                            break;
                                        }
                                    }
                                }
                            }
                            "p" => {
                                self.generate_implied_end_tags(Some("p"));
                                if self.get_current_elm_name().unwrap().as_str() != "p" {
                                    eprintln!("parse error");
                                }
                                loop {
                                    if let Some(n) = &self.stack.pop() {
                                        if let NodeType::Element(elm) = &n.borrow().node_type {
                                            if elm.tag_name == "p" {
                                                break;
                                            }
                                        }
                                    } else {
                                        bail!(ParseError {
                                            message: "p element not found".to_string(),
                                            current_token: token,
                                            current_tree: DocumentTree::build(Rc::clone(
                                                &document_node
                                            ))?
                                            .to_string(),
                                        });
                                    }
                                }
                            }
                            "li" => {
                                self.generate_implied_end_tags(Some("li"));
                                if self.get_current_elm_name().unwrap().as_str() != "li" {
                                    eprintln!("parse error");
                                }
                                loop {
                                    if let Some(n) = &self.stack.pop() {
                                        if let NodeType::Element(elm) = &n.borrow().node_type {
                                            if elm.tag_name == "li" {
                                                break;
                                            }
                                        }
                                    } else {
                                        bail!(ParseError {
                                            message: "li element not found".to_string(),
                                            current_token: token,
                                            current_tree: DocumentTree::build(Rc::clone(
                                                &document_node
                                            ))?
                                            .to_string(),
                                        });
                                    }
                                }
                            }
                            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                                self.generate_implied_end_tags(None);
                                if self.get_current_elm_name().unwrap().as_str() != tag_name {
                                    eprintln!("parse error");
                                }
                                loop {
                                    if let Some(n) = &self.stack.pop() {
                                        if let NodeType::Element(elm) = &n.borrow().node_type {
                                            if let "h1" | "h2" | "h3" | "h4" | "h5" | "h6" =
                                                elm.tag_name.as_str()
                                            {
                                                break;
                                            }
                                        }
                                    } else {
                                        bail!(ParseError {
                                            message: "h1-h6 element not found".to_string(),
                                            current_token: token,
                                            current_tree: DocumentTree::build(Rc::clone(
                                                &document_node
                                            ))?
                                            .to_string(),
                                        });
                                    }
                                }
                            }
                            _ => unimplemented!("token: {:?}", token),
                        },
                        _ => {
                            unimplemented!("token: {:?}", token);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-incdata
                    InsertionMode::Text => match &token {
                        HtmlToken::Character(c) => match c {
                            '\u{0000}' => unreachable!(),
                            _ => {
                                self.insert_char_to_token(*c);
                            }
                        },
                        HtmlToken::EndTag { tag_name, .. } if tag_name != "script" => {
                            let node = self.stack.pop().unwrap();

                            // The user agent must run the update a style block algorithm whenever any of the following conditions occur:
                            // - The element is popped off the stack of open elements of an HTML parser or XML parser.
                            // - The element is not on the stack of open elements of an HTML parser or XML parser, and it becomes connected or disconnected.
                            // - The element's children changed steps run.
                            // https://html.spec.whatwg.org/multipage/semantics.html#the-style-element
                            if tag_name == "style" {
                                self.update_style_block(node, &mut style_sheets)?;
                            }
                            self.insertion_mode = self.orig_insertion_mode.unwrap();
                        }
                        _ => {
                            unimplemented!("token: {:?}", token);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-afterbody
                    InsertionMode::AfterBody => match &token {
                        HtmlToken::Character(c) => {
                            if Self::is_blank(*c) {
                                self.insert_char_to_token(*c);
                            }
                        }
                        HtmlToken::Doctype { .. } => {
                            eprintln!("parse error, ignored the token: {:?}", token)
                        }
                        HtmlToken::EndTag { tag_name, .. } if tag_name == "html" => {
                            self.insertion_mode = InsertionMode::AfterAfterBody;
                        }
                        HtmlToken::Eof => {
                            end_of_parsing = true;
                        }
                        _ => {
                            unimplemented!("token: {:?}", token);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#the-after-after-body-insertion-mode
                    InsertionMode::AfterAfterBody => match &token {
                        HtmlToken::Doctype { .. } => {
                            eprintln!("parse error, ignored the token: {:?}", token);
                        }
                        HtmlToken::Character(c) if Self::is_blank(*c) => {
                            self.insert_char_to_token(*c);
                        }
                        HtmlToken::StartTag { tag_name, .. } if tag_name == "html" => {
                            unimplemented!("token: {:?}", token)
                        }
                        HtmlToken::Eof => {
                            end_of_parsing = true;
                        }
                        _ => {
                            eprintln!("parse error");
                            self.insertion_mode = InsertionMode::InBody;
                            continue;
                        }
                    },

                    _ => {
                        unimplemented!("token: {:?}", token);
                    }
                }

                break;
            }
        }

        Ok((document_node, style_sheets))
    }

    fn is_blank(c: char) -> bool {
        matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' ')
    }

    /// Returns the tag name of the current element, if the current node is an element.
    fn get_current_elm_name(&self) -> Option<String> {
        if let Some(node) = &self.stack.last() {
            let NodeType::Element(elm) = &node.borrow().node_type else {
                return None;
            };
            Some(elm.tag_name.clone())
        } else {
            None
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#generate-implied-end-tags
    fn generate_implied_end_tags(&mut self, excluded_tag: Option<&str>) {
        let mut tag_lists = vec![
            "dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc",
        ];
        if let Some(excluded_tag) = excluded_tag {
            let pos = tag_lists
                .iter()
                .position(|&x| x == excluded_tag)
                .unwrap_or_else(|| {
                    panic!(
                        "The tag {:?} is not in the list of tags that can be excluded",
                        excluded_tag
                    )
                });
            tag_lists.remove(pos);
        }
        loop {
            let current_node_type = self.get_current_elm_name().unwrap();
            if tag_lists.contains(&current_node_type.as_str()) {
                self.stack.pop();
            } else {
                break;
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-an-html-element
    fn insert_element(&mut self, tag_name: &str, attributes: &[(String, String)]) {
        let new_node = DomNode::append_child(
            self.stack.last().unwrap(),
            DomNode::new(NodeType::Element(Element {
                tag_name: tag_name.to_owned(),
                attributes: attributes.to_owned(),
            })),
        );
        self.stack.push(Rc::clone(&new_node));
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-a-comment
    fn insert_comment(&mut self, comment: String) {
        DomNode::append_child(
            self.stack.last().unwrap(),
            DomNode::new(NodeType::Comment(comment)),
        );
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-a-character
    fn insert_char_to_token(&mut self, c: char) {
        let mut need_to_push_node = false;
        if let Some(n) = &mut self.stack.last().unwrap().borrow_mut().child_nodes.last() {
            if let NodeType::Text(text) = &mut n.borrow_mut().node_type {
                text.push(c);
            } else {
                need_to_push_node = true;
            }
        } else {
            need_to_push_node = true;
        }

        if need_to_push_node {
            DomNode::append_child(
                self.stack.last().unwrap(),
                DomNode::new(NodeType::Text(c.to_string())),
            );
        }
    }

    /// https://html.spec.whatwg.org/multipage/semantics.html#update-a-style-block
    fn update_style_block(
        &mut self,
        node: Rc<RefCell<DomNode>>,
        style_sheets: &mut Vec<StyleSheet>,
    ) -> Result<()> {
        // When the UA should parse the CSS for the new stylesheet is not clearly defined:
        // https://github.com/whatwg/html/issues/2997
        if let NodeType::Text(css) = &node.borrow().child_nodes.last().unwrap().borrow().node_type {
            let style_sheet = CssParser::new(CssTokenizer::new(css).tokenize()?).parse()?;
            style_sheets.push(style_sheet);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse1() {
        // <!DOCTYPE html>
        // <html class=e>
        //     <head><title>Aliens?</title></head>
        //     <body>Why yes.</body>
        // </html>

        let html = "<!DOCTYPE html>\n<html class=e>\n\t<head><title>Aliens?</title></head>\n\t<body>Why yes.</body>\n</html>";
        let tree = DocumentTree::build(
            HtmlParser::new(HtmlTokenizer::new(&html))
                .parse()
                .unwrap()
                .0,
        )
        .unwrap();
        let mut dfs_iter = tree.get_dfs_iter();

        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Document
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::DocumentType("html".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "html".to_string(),
                attributes: vec![("class".to_string(), "e".to_string())],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "head".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "title".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("Aliens?".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("\n\t".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "body".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("Why yes.\n".to_string())
        );
        assert_eq!(dfs_iter.next(), None);
    }

    #[test]
    fn test_parse2() {
        // <!DOCTYPE html>
        // <html>
        //     <head><title>Lists</title></head>
        //     <body>
        //         <ul>
        //             <li>Item1
        //                 <p class="foo">Paragraph1
        //             <li>Item2</li>
        //             <li>Item3
        //         </ul>
        //     </body>
        // </html>

        let html = "<!DOCTYPE html>\n<html>\n\t<head><title>Lists</title></head>\n\t<body>\n\t\t<ul>\n\t\t\t<li>Item1\n\t\t\t\t<p class=\"foo\">Paragraph1\n\t\t\t<li>Item2</li>\n\t\t\t<li>Item3\n\t\t</ul>\n\t</body>\n</html>";
        let tree = DocumentTree::build(
            HtmlParser::new(HtmlTokenizer::new(&html))
                .parse()
                .unwrap()
                .0,
        )
        .unwrap();
        let mut dfs_iter = tree.get_dfs_iter();

        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Document
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::DocumentType("html".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "html".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "head".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "title".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("Lists".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("\n\t".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "body".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("\n\t\t".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "ul".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("\n\t\t\t".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "li".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("Item1\n\t\t\t\t".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "p".to_string(),
                attributes: vec![("class".to_string(), "foo".to_string())],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("Paragraph1\n\t\t\t".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "li".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("Item2".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("\n\t\t\t".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Element(Element {
                tag_name: "li".to_string(),
                attributes: vec![],
            })
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("Item3\n\t\t".to_string())
        );
        assert_eq!(
            dfs_iter.next().unwrap().borrow().node_type,
            NodeType::Text("\n\t\n".to_string())
        );
    }
}
