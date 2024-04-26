use std::cell::RefCell;
use std::default::Default;
use std::rc::Rc;

use anyhow::{bail, Result};
use thiserror::Error;

use crate::html::dom::{DocumentTree, Element, Node, NodeType};
use crate::html::tokenizer::{Token, Tokenizer};

#[derive(Error, Debug)]
#[error("{message} (in the Tree Construction stage)\nCurrent token: {current_token:?}\nCurrent tree:\n{current_tree}")]
struct ParseError {
    message: String,
    current_token: Token,
    current_tree: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum InsertionMode {
    #[default]
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

#[derive(Debug)]
pub struct Parser {
    insertion_mode: InsertionMode,
    tokenizer: Tokenizer,
    stack: Vec<Rc<RefCell<Node>>>,

    // When the insertion mode is switched to "text" or "in table text", the original insertion mode is also set.
    // This is the insertion mode to which the tree construction stage will return.
    orig_insertion_mode: InsertionMode,
}

impl Parser {
    pub fn new(tokenizer: Tokenizer) -> Self {
        Self {
            insertion_mode: InsertionMode::Initial,
            tokenizer,
            stack: Vec::new(),
            orig_insertion_mode: Default::default(),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#overview-of-the-parsing-model
    pub fn parse(&mut self) -> Result<Rc<RefCell<Node>>> {
        // The output of the whole parsing (tree construction) is a Document object.
        let document_node = Rc::new(RefCell::new(Node::new(NodeType::Document)));

        let mut end_of_parsing = false;
        while !end_of_parsing {
            let token = self.tokenizer.consume_token();

            loop {
                // https://html.spec.whatwg.org/multipage/parsing.html#tree-construction
                match &self.insertion_mode {
                    // https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
                    InsertionMode::Initial => {
                        match &token {
                            Token::Character(c)
                                if *c == '\t'
                                    || *c == '\n'
                                    || *c == '\x0C'
                                    || *c == '\r'
                                    || *c == ' ' =>
                            {
                                // Ignore the token
                            }
                            Token::Comment(_) => {
                                unimplemented!("token: {:?}", token);
                            }
                            Token::Doctype {
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
                                let n = Rc::new(RefCell::new(Node::new(NodeType::DocumentType(
                                    match name {
                                        Some(name) => name.clone(),
                                        None => String::new(),
                                    },
                                ))));
                                document_node.borrow_mut().child_nodes.push(Rc::clone(&n));
                                self.insertion_mode = InsertionMode::BeforeHtml;
                            }
                            _ => {
                                self.insertion_mode = InsertionMode::BeforeHtml;
                                continue; // reprocess the token
                            }
                        }
                    }

                    // https://html.spec.whatwg.org/multipage/parsing.html#the-before-html-insertion-mode
                    InsertionMode::BeforeHtml => {
                        match &token {
                            Token::Doctype { .. } => {
                                eprintln!("parse error, ignored the token: {:?}", token);
                            }
                            Token::Character(c)
                                if *c == '\t'
                                    || *c == '\n'
                                    || *c == '\x0C'
                                    || *c == '\r'
                                    || *c == ' ' =>
                            {
                                // Ignore the token
                            }
                            Token::StartTag {
                                tag_name,
                                attributes,
                                ..
                            } if tag_name == "html" => {
                                let n =
                                    Rc::new(RefCell::new(Node::new(NodeType::Element(Element {
                                        tag_name: tag_name.clone(),
                                        attributes: attributes.clone(),
                                    }))));
                                document_node.borrow_mut().child_nodes.push(Rc::clone(&n));
                                self.stack.push(Rc::clone(&n));
                                self.insertion_mode = InsertionMode::BeforeHead;
                            }
                            Token::EndTag { tag_name, .. } => {
                                if let "head" | "body" | "html" | "br" = tag_name.as_str() {
                                    let n = Rc::new(RefCell::new(Node::new(NodeType::Element(
                                        Element {
                                            tag_name: "html".to_string(),
                                            attributes: Vec::new(),
                                        },
                                    ))));
                                    document_node.borrow_mut().child_nodes.push(Rc::clone(&n));
                                    self.stack.push(Rc::clone(&n));
                                    self.insertion_mode = InsertionMode::BeforeHead;
                                } else {
                                    eprintln!("parse error, ignored the token: {:?}", token);
                                }
                            }
                            _ => {
                                let n =
                                    Rc::new(RefCell::new(Node::new(NodeType::Element(Element {
                                        tag_name: "html".to_string(),
                                        attributes: Vec::new(),
                                    }))));
                                document_node.borrow_mut().child_nodes.push(Rc::clone(&n));
                                self.stack.push(Rc::clone(&n));
                                self.insertion_mode = InsertionMode::BeforeHead;
                            }
                        }
                    }

                    // https://html.spec.whatwg.org/multipage/parsing.html#the-before-head-insertion-mode
                    InsertionMode::BeforeHead => {
                        match &token {
                            Token::Character(c)
                                if *c == '\t'
                                    || *c == '\n'
                                    || *c == '\x0C'
                                    || *c == '\r'
                                    || *c == ' ' =>
                            {
                                // Ignore the token
                            }
                            Token::Doctype { .. } => {
                                eprintln!("parse error, ignored the token: {:?}", token);
                            }
                            Token::StartTag {
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
                            Token::EndTag { tag_name, .. } => {
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
                        }
                    }

                    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
                    InsertionMode::InHead => {
                        match &token {
                            Token::Character(c)
                                if *c == '\t'
                                    || *c == '\n'
                                    || *c == '\x0C'
                                    || *c == '\r'
                                    || *c == ' ' =>
                            {
                                // Ignore the token
                            }
                            Token::Doctype { .. } => {
                                eprintln!("parse error, ignored the token: {:?}", token);
                            }
                            Token::StartTag {
                                tag_name,
                                attributes,
                                ..
                            } => match tag_name.as_str() {
                                "meta" => {
                                    let n = Rc::new(RefCell::new(Node::new(NodeType::Element(
                                        Element {
                                            tag_name: tag_name.clone(),
                                            attributes: attributes.clone(),
                                        },
                                    ))));
                                    self.stack
                                        .last()
                                        .unwrap()
                                        .borrow_mut()
                                        .child_nodes
                                        .push(Rc::clone(&n));
                                    // Insert the element (including pushing it onto the stack of open elements) and pop it immediately,
                                    // so the content of the stack is not changed here.
                                }
                                "title" => {
                                    // Quite simplified
                                    self.insert_element(tag_name, attributes);
                                    self.orig_insertion_mode = InsertionMode::InHead;
                                    self.insertion_mode = InsertionMode::Text;
                                }
                                _ => unimplemented!("token: {:?}", token),
                            },
                            Token::EndTag { tag_name, .. } if tag_name == "head" => {
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
                        Token::Character(c)
                            if *c == '\t'
                                || *c == '\n'
                                || *c == '\x0C'
                                || *c == '\r'
                                || *c == ' ' =>
                        {
                            self.insert_tokens_char(c);
                        }
                        Token::Doctype { .. } => {
                            eprintln!("parse error, ignored the token: {:?}", token);
                        }
                        Token::StartTag {
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
                        Token::Character(c) => match c {
                            '\u{0000}' => eprintln!("parse error, ignored the token: {:?}", token),
                            _ => {
                                self.insert_tokens_char(c);
                            }
                        },
                        Token::Doctype { .. } => {
                            eprintln!("parse error, ignored the token: {:?}", token)
                        }
                        Token::StartTag {
                            tag_name,
                            attributes,
                            ..
                        } => match tag_name.as_str() {
                            "div" | "p" | "ul" => {
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
                                        self.gen_implied_end_tags(Some("li"));
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
                        Token::EndTag { tag_name, .. } => match tag_name.as_str() {
                            "body" => {
                                self.insertion_mode = InsertionMode::AfterBody;
                            }
                            "div" | "ul" => {
                                self.gen_implied_end_tags(None);
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
                                self.gen_implied_end_tags(Some("p"));
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
                                self.gen_implied_end_tags(Some("li"));
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
                                self.gen_implied_end_tags(None);
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
                        Token::Character(c) => match c {
                            '\u{0000}' => unreachable!(),
                            _ => {
                                self.insert_tokens_char(c);
                            }
                        },
                        Token::EndTag { tag_name, .. } if tag_name != "script" => {
                            self.stack.pop();
                            self.insertion_mode = self.orig_insertion_mode;
                        }
                        _ => {
                            unimplemented!("token: {:?}", token);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-afterbody
                    InsertionMode::AfterBody => match &token {
                        Token::Character(c) => {
                            if let '\t' | '\n' | '\x0C' | '\r' | ' ' = c {
                                self.insert_tokens_char(c);
                            }
                        }
                        Token::Doctype { .. } => {
                            eprintln!("parse error, ignored the token: {:?}", token)
                        }
                        Token::EndTag { tag_name, .. } if tag_name == "html" => {
                            self.insertion_mode = InsertionMode::AfterAfterBody;
                        }
                        Token::Eof => {
                            end_of_parsing = true;
                        }
                        _ => {
                            unimplemented!("token: {:?}", token);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#the-after-after-body-insertion-mode
                    InsertionMode::AfterAfterBody => match &token {
                        Token::Doctype { .. } => {
                            eprintln!("parse error, ignored the token: {:?}", token);
                        }
                        Token::Character(c)
                            if *c == '\t'
                                || *c == '\n'
                                || *c == '\x0C'
                                || *c == '\r'
                                || *c == ' ' =>
                        {
                            self.insert_tokens_char(c);
                        }
                        Token::StartTag { tag_name, .. } if tag_name == "html" => {
                            unimplemented!("token: {:?}", token)
                        }
                        Token::Eof => {
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

        Ok(document_node)
    }

    /// Get the tag name of the current element, if the current node is an element.
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

    fn gen_implied_end_tags(&mut self, excluded_tag: Option<&str>) {
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

    fn insert_element(&mut self, tag_name: &str, attributes: &[(String, String)]) {
        let new_node = Rc::new(RefCell::new(Node::new(NodeType::Element(Element {
            tag_name: tag_name.to_owned(),
            attributes: attributes.to_owned(),
        }))));
        self.stack
            .last()
            .unwrap()
            .borrow_mut()
            .child_nodes
            .push(Rc::clone(&new_node));
        self.stack.push(Rc::clone(&new_node));
    }

    fn insert_tokens_char(&mut self, c: &char) {
        let mut need_to_push_node = false;
        if let Some(n) = &mut self.stack.last().unwrap().borrow_mut().child_nodes.last() {
            if let NodeType::Text(text) = &mut n.borrow_mut().node_type {
                text.push(*c);
            } else {
                need_to_push_node = true;
            }
        } else {
            need_to_push_node = true;
        }

        if need_to_push_node {
            let n = Rc::new(RefCell::new(Node::new(NodeType::Text(c.to_string()))));
            self.stack
                .last()
                .unwrap()
                .borrow_mut()
                .child_nodes
                .push(Rc::clone(&n));
        }
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
            Parser::new(Tokenizer::new(html.to_string()))
                .parse()
                .unwrap(),
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
            Parser::new(Tokenizer::new(html.to_string()))
                .parse()
                .unwrap(),
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
