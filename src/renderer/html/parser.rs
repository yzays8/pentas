use std::{cell::RefCell, rc::Rc};

use crate::{
    error::{Error, Result},
    renderer::{
        css::{CssParser, CssTokenizer, cssom::StyleSheet},
        html::{
            dom::{DomNode, Element, NodeType},
            token::{HtmlToken, HtmlTokenizer, TokenizationState},
        },
    },
};

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub struct ParsedObject {
    pub dom_root: Rc<RefCell<DomNode>>,
    pub style_sheets: Vec<StyleSheet>,
    pub title: Option<String>,
}

/// https://html.spec.whatwg.org/multipage/parsing.html#overview-of-the-parsing-model
#[derive(Debug)]
pub struct HtmlParser {
    insertion_mode: InsertionMode,
    tokenizer: HtmlTokenizer,
    stack: Vec<Rc<RefCell<DomNode>>>,
    /// When the insertion mode is switched to "text" or "in table text", the original
    /// insertion mode is also set.
    /// This is the insertion mode to which the tree construction stage will return.
    orig_insertion_mode: Option<InsertionMode>,
    output: ParsedObject,
    is_parsing_done: bool,
    is_reprocessing_required: bool,
}

impl HtmlParser {
    pub fn new(html: &str) -> Self {
        Self {
            insertion_mode: InsertionMode::Initial,
            tokenizer: HtmlTokenizer::new(html),
            stack: Vec::new(),
            orig_insertion_mode: None,
            output: ParsedObject {
                // The output of the whole parsing (tree construction) is a Document object.
                dom_root: Rc::new(RefCell::new(DomNode::new(NodeType::Document))),
                style_sheets: Vec::new(),
                title: None,
            },
            is_parsing_done: false,
            is_reprocessing_required: false,
        }
    }

    /// Returns a Document object node, CSS style sheets, and the title of the document.
    pub fn parse(&mut self) -> Result<ParsedObject> {
        while !self.is_parsing_done {
            let token = self.tokenizer.consume();

            loop {
                match &self.insertion_mode {
                    InsertionMode::Initial => self.parse_initial(&token)?,
                    InsertionMode::BeforeHtml => self.parse_before_html(&token)?,
                    InsertionMode::BeforeHead => self.parse_before_head(&token)?,
                    InsertionMode::InHead => self.parse_in_head(&token)?,
                    InsertionMode::AfterHead => self.parse_after_head(&token)?,
                    InsertionMode::InBody => self.parse_in_body(&token)?,
                    InsertionMode::Text => self.parse_text(&token)?,
                    InsertionMode::AfterBody => self.parse_after_body(&token)?,
                    InsertionMode::AfterAfterBody => self.parse_after_after_body(&token)?,
                    _ => {
                        unimplemented!("token: {:?}", token);
                    }
                }
                if !self.is_reprocessing_required {
                    break;
                }
                self.is_reprocessing_required = false;
            }
        }
        Ok(self.output.clone())
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
    fn parse_initial(&mut self, token: &HtmlToken) -> Result<()> {
        match token {
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
                        && system_identifier.clone().unwrap() != "about:legacy-compat")
                {
                    return Err(Error::HtmlParse("Invalid doctype".into()));
                }
                DomNode::append_child(
                    &self.output.dom_root,
                    DomNode::new(NodeType::DocumentType(match name {
                        Some(name) => name.clone(),
                        None => String::new(),
                    })),
                );
                self.insertion_mode = InsertionMode::BeforeHtml;
            }
            _ => {
                self.insertion_mode = InsertionMode::BeforeHtml;
                self.is_reprocessing_required = true;
            }
        }
        Ok(())
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#the-before-html-insertion-mode
    fn parse_before_html(&mut self, token: &HtmlToken) -> Result<()> {
        match token {
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
                    &self.output.dom_root,
                    DomNode::new(NodeType::Element(Element {
                        tag_name: tag_name.clone(),
                        attributes: attributes.clone(),
                    })),
                );
                self.stack.push(Rc::clone(&n));
                self.insertion_mode = InsertionMode::BeforeHead;
            }
            HtmlToken::EndTag { tag_name, .. } => {
                if matches!(tag_name.as_str(), "head" | "body" | "html" | "br") {
                    let n = DomNode::append_child(
                        &self.output.dom_root,
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
                    &self.output.dom_root,
                    DomNode::new(NodeType::Element(Element {
                        tag_name: "html".to_string(),
                        attributes: Vec::new(),
                    })),
                );
                self.stack.push(Rc::clone(&n));
                self.insertion_mode = InsertionMode::BeforeHead;
                self.is_reprocessing_required = true;
            }
        }
        Ok(())
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#the-before-head-insertion-mode
    fn parse_before_head(&mut self, token: &HtmlToken) -> Result<()> {
        match token {
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
                "html" => unimplemented!("token: {:?}", token),
                _ => {
                    self.insert_element("head", &Vec::new());
                    self.insertion_mode = InsertionMode::InHead;
                    self.is_reprocessing_required = true;
                }
            },
            HtmlToken::EndTag { tag_name, .. } => {
                if matches!(tag_name.as_str(), "head" | "body" | "html" | "br") {
                    self.insert_element(tag_name, &Vec::new());
                    self.insertion_mode = InsertionMode::InHead;
                    self.is_reprocessing_required = true;
                } else {
                    eprintln!("parse error, ignored the token: {:?}", token);
                }
            }
            _ => {
                self.insert_element("head", &Vec::new());
                self.insertion_mode = InsertionMode::InHead;
                self.is_reprocessing_required = true;
            }
        }
        Ok(())
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
    fn parse_in_head(&mut self, token: &HtmlToken) -> Result<()> {
        match token {
            HtmlToken::Character(c) if Self::is_blank(*c) => {
                self.insert_char_to_token(*c);
            }
            HtmlToken::Comment(comment) => {
                self.insert_comment(comment);
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
                "head" => {
                    eprintln!("parse error, ignored the token: {:?}", token);
                }
                "html" | "template" | "noscript" | "script" | "noframes" | "base" | "basefont"
                | "bgsound" | "link" => {
                    unimplemented!("token: {:?}", token);
                }
                _ => {
                    self.stack.pop();
                    self.insertion_mode = InsertionMode::AfterHead;
                    self.is_reprocessing_required = true;
                }
            },
            HtmlToken::EndTag { tag_name, .. } => match tag_name.as_str() {
                "head" => {
                    let elm = self.stack.pop().unwrap();
                    if let NodeType::Element(elm) = &elm.borrow().node_type {
                        if elm.tag_name != "head" {
                            return Err(Error::HtmlParse("Expected head element".into()));
                        }
                    } else {
                        return Err(Error::HtmlParse("Expected head element".into()));
                    }
                    self.insertion_mode = InsertionMode::AfterHead;
                }
                _ => unimplemented!("token: {:?}", token),
            },
            _ => {
                self.stack.pop();
                self.insertion_mode = InsertionMode::AfterHead;
                self.is_reprocessing_required = true;
            }
        }
        Ok(())
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#the-after-head-insertion-mode
    fn parse_after_head(&mut self, token: &HtmlToken) -> Result<()> {
        match token {
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
                "head" => {
                    eprintln!("parse error, ignored the token: {:?}", token);
                }
                "html" | "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes"
                | "script" | "style" | "template" | "title" => {
                    unimplemented!("token: {:?}", token);
                }
                _ => {
                    self.insert_element("body", &Vec::new());
                    self.insertion_mode = InsertionMode::InBody;
                    self.is_reprocessing_required = true;
                }
            },
            HtmlToken::EndTag { .. } => unimplemented!("token: {:?}", token),
            _ => {
                self.insert_element("body", &Vec::new());
                self.insertion_mode = InsertionMode::InBody;
                self.is_reprocessing_required = true;
            }
        }
        Ok(())
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inbody
    fn parse_in_body(&mut self, token: &HtmlToken) -> Result<()> {
        match token {
            HtmlToken::Character(c) => match c {
                '\u{0000}' => eprintln!("parse error, ignored the token: {:?}", token),
                _ => {
                    self.insert_char_to_token(*c);
                }
            },
            HtmlToken::Comment(comment) => {
                self.insert_comment(comment);
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
                    if let Some(n) = &self.stack.last()
                        && let NodeType::Element(elm) = &n.borrow().node_type
                        && matches!(
                            elm.tag_name.as_str(),
                            "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                        )
                    {
                        eprintln!("parse error");
                        need_to_pop = true;
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
                                    if let NodeType::Element(elm) = &n.borrow().node_type
                                        && elm.tag_name == "li"
                                    {
                                        break;
                                    }
                                } else {
                                    return Err(Error::HtmlParse("li element not found".into()));
                                }
                            }
                            break;
                        }

                        // This is quite simplified.
                        if let "address" | "div" | "p" = node_type.as_str() {
                            // Set the node to the previous entry in the stack of open elements.
                            node_type = if let Some(node) = &self.stack.get(self.stack.len() - 2) {
                                let NodeType::Element(elm) = &node.borrow().node_type else {
                                    return Err(Error::HtmlParse(
                                        "The node is not an element".into(),
                                    ));
                                };
                                elm.tag_name.clone()
                            } else {
                                return Err(Error::HtmlParse("Previous node not found".into()));
                            };
                            continue;
                        } else {
                            // If node is in the special category, but is not an address, div, or p element
                            break;
                        }
                    }
                    self.insert_element(tag_name, attributes);
                }
                "br" => {
                    self.insert_element(tag_name, attributes);
                    self.stack.pop();
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
                            if let NodeType::Element(elm) = &n.borrow().node_type
                                && elm.tag_name == "a"
                            {
                                break;
                            }
                        } else {
                            return Err(Error::HtmlParse("a element not found".into()));
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
                        if let NodeType::Element(elm) = &n.borrow().node_type
                            && elm.tag_name == *tag_name
                        {
                            break;
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
                            if let NodeType::Element(elm) = &n.borrow().node_type
                                && elm.tag_name == "p"
                            {
                                break;
                            }
                        } else {
                            return Err(Error::HtmlParse("p element not found".into()));
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
                            if let NodeType::Element(elm) = &n.borrow().node_type
                                && elm.tag_name == "li"
                            {
                                break;
                            }
                        } else {
                            return Err(Error::HtmlParse("li element not found".into()));
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
                            if let NodeType::Element(elm) = &n.borrow().node_type
                                && matches!(
                                    elm.tag_name.as_str(),
                                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                                )
                            {
                                break;
                            }
                        } else {
                            return Err(Error::HtmlParse("h1-h6 element not found".into()));
                        }
                    }
                }
                _ => unimplemented!("token: {:?}", token),
            },
            HtmlToken::Eof => {
                self.is_parsing_done = true;
            }
        }
        Ok(())
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-incdata
    fn parse_text(&mut self, token: &HtmlToken) -> Result<()> {
        match token {
            HtmlToken::Character(c) => match c {
                '\u{0000}' => unreachable!(),
                _ => {
                    self.insert_char_to_token(*c);
                }
            },
            HtmlToken::EndTag { tag_name, .. } if tag_name != "script" => {
                let node = self.stack.pop().unwrap();

                if tag_name == "title" {
                    self.output.title = Some(
                        node.borrow()
                            .children
                            .first()
                            .unwrap()
                            .borrow()
                            .get_inside_text()
                            .unwrap(),
                    );
                }

                // The user agent must run the update a style block algorithm whenever any of the following conditions occur:
                // - The element is popped off the stack of open elements of an HTML parser or XML parser.
                // - The element is not on the stack of open elements of an HTML parser or XML parser, and it becomes connected or disconnected.
                // - The element's children changed steps run.
                // https://html.spec.whatwg.org/multipage/semantics.html#the-style-element
                if tag_name == "style" {
                    self.update_style_block(&node)?;
                }

                self.insertion_mode = self.orig_insertion_mode.unwrap();
            }
            _ => {
                unimplemented!("token: {:?}", token);
            }
        }
        Ok(())
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-afterbody
    fn parse_after_body(&mut self, token: &HtmlToken) -> Result<()> {
        match token {
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
                self.is_parsing_done = true;
            }
            _ => {
                unimplemented!("token: {:?}", token);
            }
        }
        Ok(())
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#the-after-after-body-insertion-mode
    fn parse_after_after_body(&mut self, token: &HtmlToken) -> Result<()> {
        match token {
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
                self.is_parsing_done = true;
            }
            _ => {
                eprintln!("parse error");
                self.insertion_mode = InsertionMode::InBody;
                self.is_parsing_done = true;
            }
        }
        Ok(())
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
    fn insert_comment(&mut self, comment: &str) {
        DomNode::append_child(
            self.stack.last().unwrap(),
            DomNode::new(NodeType::Comment(comment.to_string())),
        );
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-a-character
    fn insert_char_to_token(&mut self, c: char) {
        let mut need_to_push_node = false;
        if let Some(n) = &mut self.stack.last().unwrap().borrow_mut().children.last() {
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
    fn update_style_block(&mut self, node: &Rc<RefCell<DomNode>>) -> Result<()> {
        // When the UA should parse the CSS for the new stylesheet is not clearly defined:
        // https://github.com/whatwg/html/issues/2997
        if let NodeType::Text(css) = &node.borrow().children.last().unwrap().borrow().node_type {
            let style_sheet = CssParser::new(&CssTokenizer::new(css).tokenize()?).parse()?;
            self.output.style_sheets.push(style_sheet);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::html::dom::DocumentTree;

    // <!DOCTYPE html>
    // <html class=e>
    //     <head><title>Aliens?</title></head>
    //     <body>Why yes.</body>
    // </html>
    #[test]
    fn parse_simple_html() {
        let html = "<!DOCTYPE html>\n<html class=e>\n\t<head><title>Aliens?</title></head>\n\t<body>Why yes.</body>\n</html>";
        let tree = DocumentTree::build(HtmlParser::new(html).parse().unwrap().dom_root).unwrap();
        let actual = tree
            .get_dfs_iter()
            .map(|node| node.borrow().node_type.clone())
            .collect::<Vec<_>>();
        let expected = vec![
            NodeType::Document,
            NodeType::DocumentType("html".to_string()),
            NodeType::Element(Element {
                tag_name: "html".to_string(),
                attributes: vec![("class".to_string(), "e".to_string())],
            }),
            NodeType::Element(Element {
                tag_name: "head".to_string(),
                attributes: vec![],
            }),
            NodeType::Element(Element {
                tag_name: "title".to_string(),
                attributes: vec![],
            }),
            NodeType::Text("Aliens?".to_string()),
            NodeType::Text("\n\t".to_string()),
            NodeType::Element(Element {
                tag_name: "body".to_string(),
                attributes: vec![],
            }),
            NodeType::Text("Why yes.\n".to_string()),
        ];

        assert_eq!(actual, expected);
    }

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
    #[test]
    fn parse_li_without_end_tag() {
        let html = "<!DOCTYPE html>\n<html>\n\t<head><title>Lists</title></head>\n\t<body>\n\t\t<ul>\n\t\t\t<li>Item1\n\t\t\t\t\
        <p class=\"foo\">Paragraph1\n\t\t\t<li>Item2</li>\n\t\t\t<li>Item3\n\t\t</ul>\n\t</body>\n</html>";
        let tree = DocumentTree::build(HtmlParser::new(html).parse().unwrap().dom_root).unwrap();
        let actual = tree
            .get_dfs_iter()
            .map(|node| node.borrow().node_type.clone())
            .collect::<Vec<_>>();
        let expected = vec![
            NodeType::Document,
            NodeType::DocumentType("html".to_string()),
            NodeType::Element(Element {
                tag_name: "html".to_string(),
                attributes: vec![],
            }),
            NodeType::Element(Element {
                tag_name: "head".to_string(),
                attributes: vec![],
            }),
            NodeType::Element(Element {
                tag_name: "title".to_string(),
                attributes: vec![],
            }),
            NodeType::Text("Lists".to_string()),
            NodeType::Text("\n\t".to_string()),
            NodeType::Element(Element {
                tag_name: "body".to_string(),
                attributes: vec![],
            }),
            NodeType::Text("\n\t\t".to_string()),
            NodeType::Element(Element {
                tag_name: "ul".to_string(),
                attributes: vec![],
            }),
            NodeType::Text("\n\t\t\t".to_string()),
            NodeType::Element(Element {
                tag_name: "li".to_string(),
                attributes: vec![],
            }),
            NodeType::Text("Item1\n\t\t\t\t".to_string()),
            NodeType::Element(Element {
                tag_name: "p".to_string(),
                attributes: vec![("class".to_string(), "foo".to_string())],
            }),
            NodeType::Text("Paragraph1\n\t\t\t".to_string()),
            NodeType::Element(Element {
                tag_name: "li".to_string(),
                attributes: vec![],
            }),
            NodeType::Text("Item2".to_string()),
            NodeType::Text("\n\t\t\t".to_string()),
            NodeType::Element(Element {
                tag_name: "li".to_string(),
                attributes: vec![],
            }),
            NodeType::Text("Item3\n\t\t".to_string()),
            NodeType::Text("\n\t\n".to_string()),
        ];

        assert_eq!(actual, expected);
    }

    // <h1>Heading</h1>
    // <p>paragraph
    #[test]
    fn parse_incomplete_html() {
        let html = "<h1>heading</h1>\n<p>paragraph</p>";
        let tree = DocumentTree::build(HtmlParser::new(html).parse().unwrap().dom_root).unwrap();
        let actual = tree
            .get_dfs_iter()
            .map(|node| node.borrow().node_type.clone())
            .collect::<Vec<_>>();
        let expected = vec![
            NodeType::Document,
            NodeType::Element(Element {
                tag_name: "html".to_string(),
                attributes: vec![],
            }),
            NodeType::Element(Element {
                tag_name: "head".to_string(),
                attributes: vec![],
            }),
            NodeType::Element(Element {
                tag_name: "body".to_string(),
                attributes: vec![],
            }),
            NodeType::Element(Element {
                tag_name: "h1".to_string(),
                attributes: vec![],
            }),
            NodeType::Text("heading".to_string()),
            NodeType::Text("\n".to_string()),
            NodeType::Element(Element {
                tag_name: "p".to_string(),
                attributes: vec![],
            }),
            NodeType::Text("paragraph".to_string()),
        ];

        assert_eq!(actual, expected);
    }
}
