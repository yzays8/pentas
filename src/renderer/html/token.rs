use std::collections::VecDeque;

use crate::utils::TokenIterator;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TokenizationState {
    #[default]
    Data,
    RawText,
    TagOpen,
    EndTagOpen,
    TagName,
    RawTextLessThanSign,
    RawTextEndTagOpen,
    RawTextEndTagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    SelfClosingStartTag,
    AfterAttributeValueQuoted,
    BogusComment,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentEndDash,
    CommentEnd,
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
}

/// The output of the tokenization step is a series of zero or more of the following tokens:
/// DOCTYPE, start tag, end tag, comment, character, end-of-file.
#[derive(Debug, Clone, PartialEq)]
pub enum HtmlToken {
    Doctype {
        /// This is marked as missing when the token is created.
        name: Option<String>,
        /// This is marked as missing when the token is created.
        public_identifier: Option<String>,
        /// This is marked as missing when the token is created.
        system_identifier: Option<String>,
        /// This is set to off when the token is created.
        force_quirks: bool,
    },
    StartTag {
        tag_name: String,
        /// A list of attributes, where each attribute is a tuple of (name, value).
        /// This is empty when the token is created.
        attributes: Vec<(String, String)>,
        /// This is unset when the token is created.
        self_closing: bool,
    },
    EndTag {
        tag_name: String,
        /// A list of attributes, where each attribute is a tuple of (name, value).
        /// This is empty when the token is created.
        attributes: Vec<(String, String)>,
        /// This is unset when the token is created.
        self_closing: bool,
    },
    Comment(String),
    Character(char),
    Eof,
}

#[derive(Debug)]
pub struct HtmlTokenizer {
    state: TokenizationState,
    current_token: Option<HtmlToken>,
    /// The tree construction stage can affect the state of the tokenization stage,
    /// and can insert additional characters into the stream.
    /// However, for simplicity, we assume that the input stream is never modified.
    input: TokenIterator<char>,
    output: VecDeque<HtmlToken>,
    /// https://html.spec.whatwg.org/multipage/parsing.html#temporary-buffer
    temp_buf: Vec<char>,
}

impl HtmlTokenizer {
    pub fn new(html: &str) -> Self {
        Self {
            state: TokenizationState::Data,
            current_token: None,
            input: TokenIterator::new(&html.chars().collect::<Vec<_>>()),
            output: VecDeque::new(),
            temp_buf: Vec::new(),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#reconsume
    fn allow_reconsume(&mut self, new_state: TokenizationState) {
        self.state = new_state;
        self.input.rewind(1);
    }

    fn create_token(&mut self, token: HtmlToken) {
        self.current_token = Some(token);
    }

    fn emit_token(&mut self, token: HtmlToken) {
        self.output.push_back(token);
    }

    fn emit_tokens(&mut self, tokens: Vec<HtmlToken>) {
        for token in tokens {
            self.emit_token(token);
        }
    }

    /// This is expected to be called by the HTML parser.
    pub fn change_state(&mut self, new_state: TokenizationState) {
        self.state = new_state;
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#tokenization
    pub fn consume(&mut self) -> HtmlToken {
        // Buffer the output since multiple tokens can be emitted at once.
        while self.output.is_empty() {
            match self.state {
                TokenizationState::Data => self.tokenize_data(),
                TokenizationState::RawText => self.tokenize_rawtext(),
                TokenizationState::TagOpen => self.tokenize_tag_open(),
                TokenizationState::EndTagOpen => self.tokenize_end_tag_open(),
                TokenizationState::TagName => self.tokenize_tag_name(),
                TokenizationState::RawTextLessThanSign => self.tokenize_rawtext_less_than_sign(),
                TokenizationState::RawTextEndTagOpen => self.tokenize_rawtext_end_tag_open(),
                TokenizationState::RawTextEndTagName => self.tokenize_rawtext_end_tag_name(),
                TokenizationState::BeforeAttributeName => self.tokenize_before_attr_name(),
                TokenizationState::AttributeName => self.tokenize_attr_name(),
                TokenizationState::AfterAttributeName => self.tokenize_after_attr_name(),
                TokenizationState::BeforeAttributeValue => self.tokenize_before_attr_value(),
                TokenizationState::AttributeValueDoubleQuoted => {
                    self.tokenize_attr_value_double_quoted()
                }
                TokenizationState::AttributeValueSingleQuoted => {
                    self.tokenize_attr_value_single_quoted()
                }
                TokenizationState::AttributeValueUnquoted => self.tokenize_attr_value_unquoted(),
                TokenizationState::AfterAttributeValueQuoted => {
                    self.tokenize_after_attr_value_quoted()
                }
                TokenizationState::SelfClosingStartTag => self.tokenize_self_closing_start_tag(),
                TokenizationState::BogusComment => self.tokenize_bogus_comment(),
                TokenizationState::MarkupDeclarationOpen => self.tokenize_markup_declaration_open(),
                TokenizationState::CommentStart => self.tokenize_comment_start(),
                TokenizationState::CommentStartDash => self.tokenize_comment_start_dash(),
                TokenizationState::Comment => self.tokenize_comment(),
                TokenizationState::CommentEndDash => self.tokenize_comment_end_dash(),
                TokenizationState::CommentEnd => self.tokenize_comment_end(),
                TokenizationState::Doctype => self.tokenize_doctype(),
                TokenizationState::BeforeDoctypeName => self.tokenize_before_doctype_name(),
                TokenizationState::DoctypeName => self.tokenize_doctype_name(),
                TokenizationState::AfterDoctypeName => self.tokenize_after_doctype_name(),
            }
        }
        assert!(!self.output.is_empty());
        self.output.pop_front().unwrap()
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#data-state
    fn tokenize_data(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '&' => {
                    unimplemented!();
                }
                '<' => {
                    self.state = TokenizationState::TagOpen;
                }
                '\u{0000}' => {
                    eprintln!("unexpected-null-character parse error");
                    self.emit_token(HtmlToken::Character('\u{FFFD}'));
                }
                _ => {
                    self.emit_token(HtmlToken::Character(c));
                }
            },
            None => {
                self.emit_token(HtmlToken::Eof);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rawtext-state
    fn tokenize_rawtext(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '<' => {
                    self.state = TokenizationState::RawTextLessThanSign;
                }
                '\u{0000}' => {
                    eprintln!("unexpected-null-character parse error");
                    self.emit_token(HtmlToken::Character('\u{FFFD}'));
                }
                _ => {
                    self.emit_token(HtmlToken::Character(c));
                }
            },
            None => {
                self.emit_token(HtmlToken::Eof);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
    fn tokenize_tag_open(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '!' => {
                    self.state = TokenizationState::MarkupDeclarationOpen;
                }
                '/' => {
                    self.state = TokenizationState::EndTagOpen;
                }
                c if c.is_ascii_alphabetic() => {
                    self.create_token(HtmlToken::StartTag {
                        tag_name: String::new(),
                        attributes: vec![],
                        self_closing: false,
                    });
                    self.allow_reconsume(TokenizationState::TagName);
                }
                _ => {
                    eprintln!("invalid-first-character-of-tag-name parse error");
                    self.allow_reconsume(TokenizationState::Data);
                    self.emit_token(HtmlToken::Character('<'));
                }
            },
            None => {
                eprintln!("eof-before-tag-name parse error");
                self.emit_tokens(vec![HtmlToken::Character('<'), HtmlToken::Eof]);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#end-tag-open-state
    fn tokenize_end_tag_open(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                c if c.is_ascii_alphabetic() => {
                    self.create_token(HtmlToken::EndTag {
                        tag_name: String::new(),
                        attributes: vec![],
                        self_closing: false,
                    });
                    self.allow_reconsume(TokenizationState::TagName);
                }
                '>' => {
                    eprintln!("missing-end-tag-name parse error");
                    self.state = TokenizationState::Data;
                }
                _ => {
                    eprintln!("invalid-first-character-of-tag-name parse error");
                    self.create_token(HtmlToken::Comment(String::new()));
                    self.allow_reconsume(TokenizationState::BogusComment);
                }
            },
            None => {
                eprintln!("eof-before-tag-name parse error");
                self.emit_tokens(vec![
                    HtmlToken::Character('<'),
                    HtmlToken::Character('/'),
                    HtmlToken::Eof,
                ]);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#tag-name-state
    fn tokenize_tag_name(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' => {
                    self.state = TokenizationState::BeforeAttributeName;
                }
                '/' => {
                    self.state = TokenizationState::SelfClosingStartTag;
                }
                '>' => {
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                c if c.is_ascii_uppercase() => {
                    if let Some(
                        HtmlToken::StartTag { tag_name, .. } | HtmlToken::EndTag { tag_name, .. },
                    ) = &mut self.current_token
                    {
                        tag_name.push(c.to_ascii_lowercase());
                    }
                }
                '\u{0000}' => {
                    if let Some(
                        HtmlToken::StartTag { tag_name, .. } | HtmlToken::EndTag { tag_name, .. },
                    ) = &mut self.current_token
                    {
                        eprintln!("unexpected-null-character parse error");
                        tag_name.push('\u{FFFD}');
                    }
                }
                _ => {
                    if let Some(
                        HtmlToken::StartTag { tag_name, .. } | HtmlToken::EndTag { tag_name, .. },
                    ) = &mut self.current_token
                    {
                        tag_name.push(c);
                    }
                }
            },
            None => {
                eprintln!("eof-in-tag parse error");
                self.emit_token(HtmlToken::Eof);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rawtext-less-than-sign-state
    fn tokenize_rawtext_less_than_sign(&mut self) {
        match self.input.next() {
            Some('/') => {
                self.temp_buf.clear();
                self.state = TokenizationState::RawTextEndTagOpen;
            }
            _ => {
                self.allow_reconsume(TokenizationState::RawText);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rawtext-end-tag-open-state
    fn tokenize_rawtext_end_tag_open(&mut self) {
        match self.input.next() {
            Some(c) if c.is_ascii_alphabetic() => {
                self.create_token(HtmlToken::EndTag {
                    tag_name: String::new(),
                    attributes: vec![],
                    self_closing: false,
                });
                self.allow_reconsume(TokenizationState::RawTextEndTagName);
            }
            _ => {
                self.emit_tokens(vec![HtmlToken::Character('<'), HtmlToken::Character('/')]);
                self.allow_reconsume(TokenizationState::RawText);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rawtext-end-tag-name-state
    fn tokenize_rawtext_end_tag_name(&mut self) {
        match self.input.next() {
            Some('\t' | '\n' | '\x0C' | ' ') => {
                // todo: check if the current end tag token's tag name is an appropriate end tag name
                self.state = TokenizationState::BeforeAttributeName;
            }
            Some('/') => {
                // todo: check if the current end tag token's tag name is an appropriate end tag name
                self.state = TokenizationState::SelfClosingStartTag;
            }
            Some('>') => {
                // todo: check if the current end tag token's tag name is an appropriate end tag name
                self.state = TokenizationState::Data;
                self.emit_token(self.current_token.clone().unwrap());
            }
            Some(c) if c.is_ascii() => {
                if let Some(HtmlToken::EndTag { tag_name, .. }) = &mut self.current_token {
                    tag_name.push(c.to_ascii_lowercase());
                }
                self.temp_buf.push(c);
            }
            _ => {
                self.emit_tokens(vec![HtmlToken::Character('<'), HtmlToken::Character('/')]);
                for c in &self.temp_buf {
                    self.output.push_back(HtmlToken::Character(*c));
                }
                self.allow_reconsume(TokenizationState::RawText);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
    fn tokenize_before_attr_name(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' => {}
                '/' | '>' => {
                    self.allow_reconsume(TokenizationState::AfterAttributeName);
                }
                '=' => {
                    eprintln!("unexpected-equals-sign-before-attribute-name parse error");

                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        attributes.push((c.to_string(), String::new()));
                    }
                }
                _ => {
                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        attributes.push((String::new(), String::new()));
                    }
                    self.allow_reconsume(TokenizationState::AttributeName);
                }
            },
            None => {
                self.allow_reconsume(TokenizationState::AfterAttributeName);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-name-state
    fn tokenize_attr_name(&mut self) {
        match self.input.next() {
            // todo: UA needs to check if current attribute's name is already in the attributes list before leaving this state
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' | '/' | '>' => {
                    self.allow_reconsume(TokenizationState::AfterAttributeName);
                }
                '=' => {
                    self.state = TokenizationState::BeforeAttributeValue;
                }
                c if c.is_ascii_uppercase() => match &mut self.current_token {
                    Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) => {
                        attributes
                            .last_mut()
                            .unwrap()
                            .0
                            .push(c.to_ascii_lowercase());
                    }
                    _ => {
                        unreachable!();
                    }
                },
                '\u{0000}' => {
                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        eprintln!("unexpected-null-character parse error");
                        attributes.last_mut().unwrap().0.push('\u{FFFD}');
                    }
                }
                _ => {
                    if matches!(c, '"' | '\'' | '<') {
                        eprintln!("invalid-character-in-attribute-name parse error");
                    }
                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        attributes.last_mut().unwrap().0.push(c);
                    }
                }
            },
            None => {
                self.allow_reconsume(TokenizationState::AfterAttributeName);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-name-state
    fn tokenize_after_attr_name(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' => {}
                '/' => {
                    self.state = TokenizationState::SelfClosingStartTag;
                }
                '=' => {
                    self.state = TokenizationState::BeforeAttributeValue;
                }
                '>' => {
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                _ => {
                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        attributes.push((String::new(), String::new()));
                    }
                    self.allow_reconsume(TokenizationState::AttributeName);
                }
            },
            None => {
                eprintln!("eof-in-tag parse error");
                self.emit_token(HtmlToken::Eof);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-value-state
    fn tokenize_before_attr_value(&mut self) {
        match self.input.next() {
            Some('\t' | '\n' | '\x0C' | ' ') => {}
            Some('"') => {
                self.state = TokenizationState::AttributeValueDoubleQuoted;
            }
            Some('\'') => {
                self.state = TokenizationState::AttributeValueSingleQuoted;
            }
            Some('>') => {
                eprintln!("missing-attribute-value parse error");
                self.state = TokenizationState::Data;
                self.emit_token(self.current_token.clone().unwrap());
            }
            _ => {
                self.allow_reconsume(TokenizationState::AttributeValueUnquoted);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(double-quoted)-state
    fn tokenize_attr_value_double_quoted(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '"' => {
                    self.state = TokenizationState::AfterAttributeValueQuoted;
                }
                '&' => {
                    unimplemented!();
                }
                '\u{0000}' => {
                    eprintln!("unexpected-null-character parse error");
                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        attributes.last_mut().unwrap().1.push('\u{FFFD}');
                    }
                }
                _ => {
                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        attributes.last_mut().unwrap().1.push(c);
                    }
                }
            },
            None => {
                eprintln!("eof-in-tag parse error");
                self.emit_token(HtmlToken::Eof);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(single-quoted)-state
    fn tokenize_attr_value_single_quoted(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '\'' => {
                    self.state = TokenizationState::AfterAttributeValueQuoted;
                }
                '&' => {
                    unimplemented!();
                }
                '\u{0000}' => {
                    eprintln!("unexpected-null-character parse error");
                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        attributes.last_mut().unwrap().1.push('\u{FFFD}');
                    }
                }
                _ => {
                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        attributes.last_mut().unwrap().1.push(c);
                    }
                }
            },
            None => {
                eprintln!("eof-in-tag parse error");
                self.emit_token(HtmlToken::Eof);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(unquoted)-state
    fn tokenize_attr_value_unquoted(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' => {
                    self.state = TokenizationState::BeforeAttributeName;
                }
                '&' => {
                    unimplemented!();
                }
                '>' => {
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                '\u{0000}' => {
                    eprintln!("unexpected-null-character parse error");
                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        attributes.last_mut().unwrap().1.push('\u{FFFD}');
                    }
                }
                _ => {
                    if matches!(c, '"' | '\'' | '<' | '=' | '`') {
                        eprintln!("unexpected-character-in-unquoted-attribute-value parse error");
                    }
                    if let Some(
                        HtmlToken::StartTag { attributes, .. }
                        | HtmlToken::EndTag { attributes, .. },
                    ) = &mut self.current_token
                    {
                        attributes.last_mut().unwrap().1.push(c);
                    }
                }
            },
            None => {
                eprintln!("eof-in-tag parse error");
                self.emit_token(HtmlToken::Eof);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-value-(quoted)-state
    fn tokenize_after_attr_value_quoted(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' => {
                    self.state = TokenizationState::BeforeAttributeName;
                }
                '/' => {
                    self.state = TokenizationState::SelfClosingStartTag;
                }
                '>' => {
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                _ => {
                    eprintln!("missing-whitespace-between-attributes parse error");
                    self.allow_reconsume(TokenizationState::BeforeAttributeName);
                }
            },
            None => {
                eprintln!("eof-in-tag parse error");
                self.emit_token(HtmlToken::Eof);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#self-closing-start-tag-state
    fn tokenize_self_closing_start_tag(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '>' => {
                    if let Some(
                        HtmlToken::StartTag { self_closing, .. }
                        | HtmlToken::EndTag { self_closing, .. },
                    ) = &mut self.current_token
                    {
                        *self_closing = true;
                    }
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                _ => {
                    eprintln!("unexpected-solidus-in-tag parse error");
                    self.allow_reconsume(TokenizationState::BeforeAttributeName);
                }
            },
            None => {
                eprintln!("eof-in-tag parse error");
                self.emit_token(HtmlToken::Eof);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#bogus-comment-state
    fn tokenize_bogus_comment(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '>' => {
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                '\u{0000}' => {
                    eprintln!("unexpected-null-character parse error");
                    if let Some(HtmlToken::Comment(comment)) = &mut self.current_token {
                        comment.push('\u{FFFD}');
                    }
                }
                _ => {
                    if let Some(HtmlToken::Comment(comment)) = &mut self.current_token {
                        comment.push(c);
                    }
                }
            },
            None => {
                self.emit_tokens(vec![self.current_token.clone().unwrap(), HtmlToken::Eof]);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#markup-declaration-open-state
    fn tokenize_markup_declaration_open(&mut self) {
        if self
            .input
            .peek_chunk(2)
            .iter()
            .flatten()
            .copied()
            .collect::<String>()
            .eq_ignore_ascii_case("--")
        {
            self.input.forward(2);
            self.create_token(HtmlToken::Comment(String::new()));
            self.state = TokenizationState::CommentStart;
        } else if self
            .input
            .peek_chunk(7)
            .iter()
            .flatten()
            .copied()
            .collect::<String>()
            .eq_ignore_ascii_case("DOCTYPE")
        {
            self.input.forward(7);
            self.state = TokenizationState::Doctype;
        } else if self
            .input
            .peek_chunk(7)
            .iter()
            .flatten()
            .copied()
            .collect::<String>()
            .eq_ignore_ascii_case("[CDATA[")
        {
            unimplemented!();
        } else {
            eprintln!("incorrectly-opened-comment parse error");
            self.create_token(HtmlToken::Comment(String::new()));
            self.state = TokenizationState::BogusComment;
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#comment-start-state
    fn tokenize_comment_start(&mut self) {
        match self.input.next() {
            Some('-') => {
                self.state = TokenizationState::CommentStartDash;
            }
            Some('>') => {
                eprintln!("abrupt-closing-of-empty-comment parse error");
                self.state = TokenizationState::Data;
                self.emit_token(self.current_token.clone().unwrap());
            }
            _ => {
                self.allow_reconsume(TokenizationState::Comment);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#comment-start-dash-state
    fn tokenize_comment_start_dash(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '-' => {
                    self.state = TokenizationState::CommentEnd;
                }
                '>' => {
                    eprintln!("abrupt-closing-of-empty-comment parse error");
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                _ => {
                    if let Some(HtmlToken::Comment(comment)) = &mut self.current_token {
                        comment.push('-');
                    }
                    self.allow_reconsume(TokenizationState::Comment);
                }
            },
            None => {
                eprintln!("eof-in-comment parse error");
                self.emit_tokens(vec![self.current_token.clone().unwrap(), HtmlToken::Eof]);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#comment-state
    fn tokenize_comment(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '<' => {
                    if let Some(HtmlToken::Comment(comment)) = &mut self.current_token {
                        comment.push(c);
                    }
                    todo!()
                }
                '-' => {
                    self.state = TokenizationState::CommentEndDash;
                }
                '\u{0000}' => {
                    eprintln!("unexpected-null-character parse error");
                    if let Some(HtmlToken::Comment(comment)) = &mut self.current_token {
                        comment.push('\u{FFFD}');
                    }
                }
                _ => {
                    if let Some(HtmlToken::Comment(comment)) = &mut self.current_token {
                        comment.push(c);
                    }
                }
            },
            None => {
                eprintln!("eof-in-comment parse error");
                self.emit_tokens(vec![self.current_token.clone().unwrap(), HtmlToken::Eof]);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#comment-end-dash-state
    fn tokenize_comment_end_dash(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '-' => {
                    self.state = TokenizationState::CommentEnd;
                }
                _ => {
                    if let Some(HtmlToken::Comment(comment)) = &mut self.current_token {
                        comment.push('-');
                    }
                    self.allow_reconsume(TokenizationState::Comment);
                }
            },
            None => {
                eprintln!("eof-in-comment parse error");
                self.emit_tokens(vec![self.current_token.clone().unwrap(), HtmlToken::Eof]);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#comment-end-state
    fn tokenize_comment_end(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '>' => {
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                '!' => {
                    unimplemented!();
                }
                '-' => {
                    if let Some(HtmlToken::Comment(comment)) = &mut self.current_token {
                        comment.push('-');
                    }
                }
                _ => {
                    if let Some(HtmlToken::Comment(comment)) = &mut self.current_token {
                        comment.push('-');
                        comment.push('-');
                        comment.push(c);
                    }
                    self.allow_reconsume(TokenizationState::Comment);
                }
            },
            None => {
                eprintln!("eof-in-comment parse error");
                self.emit_tokens(vec![self.current_token.clone().unwrap(), HtmlToken::Eof]);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#doctype-state
    fn tokenize_doctype(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' => {
                    self.state = TokenizationState::BeforeDoctypeName;
                }
                '>' => {
                    self.allow_reconsume(TokenizationState::BeforeDoctypeName);
                }
                _ => {
                    eprintln!("missing-whitespace-before-doctype-name parse error");
                    self.allow_reconsume(TokenizationState::BeforeDoctypeName);
                }
            },
            None => {
                eprintln!("eof-in-doctype parse error");
                self.create_token(HtmlToken::Doctype {
                    name: None,
                    public_identifier: None,
                    system_identifier: None,
                    force_quirks: true,
                });
                self.emit_tokens(vec![self.current_token.clone().unwrap(), HtmlToken::Eof]);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#before-doctype-name-state
    fn tokenize_before_doctype_name(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' => {}
                c if c.is_ascii_uppercase() => {
                    self.create_token(HtmlToken::Doctype {
                        name: c.to_ascii_lowercase().to_string().into(),
                        public_identifier: None,
                        system_identifier: None,
                        force_quirks: false,
                    });
                    self.state = TokenizationState::DoctypeName;
                }
                '\u{0000}' => {
                    eprintln!("unexpected-null-character parse error");
                    self.create_token(HtmlToken::Doctype {
                        name: '\u{FFFD}'.to_string().into(),
                        public_identifier: None,
                        system_identifier: None,
                        force_quirks: false,
                    });
                    self.state = TokenizationState::DoctypeName;
                }
                '>' => {
                    eprintln!("missing-doctype-name parse error");
                    self.create_token(HtmlToken::Doctype {
                        name: None,
                        public_identifier: None,
                        system_identifier: None,
                        force_quirks: true,
                    });
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                _ => {
                    self.create_token(HtmlToken::Doctype {
                        name: c.to_string().into(),
                        public_identifier: None,
                        system_identifier: None,
                        force_quirks: false,
                    });
                    self.state = TokenizationState::DoctypeName;
                }
            },
            None => {
                eprintln!("eof-in-doctype parse error");
                self.create_token(HtmlToken::Doctype {
                    name: None,
                    public_identifier: None,
                    system_identifier: None,
                    force_quirks: true,
                });
                self.emit_tokens(vec![self.current_token.clone().unwrap(), HtmlToken::Eof]);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#doctype-name-state
    fn tokenize_doctype_name(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' => {
                    self.state = TokenizationState::AfterDoctypeName;
                }
                '>' => {
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                c if c.is_ascii_uppercase() => match &mut self.current_token {
                    Some(HtmlToken::Doctype { name, .. }) => {
                        if let Some(n) = name {
                            n.push(c.to_ascii_lowercase());
                        }
                    }
                    _ => {
                        unreachable!();
                    }
                },
                '\u{0000}' => {
                    eprintln!("unexpected-null-character parse error");
                    if let Some(HtmlToken::Doctype { name: Some(n), .. }) = &mut self.current_token
                    {
                        n.push('\u{FFFD}');
                    }
                }
                _ => {
                    if let Some(HtmlToken::Doctype { name: Some(n), .. }) = &mut self.current_token
                    {
                        n.push(c);
                    }
                }
            },
            None => {
                eprintln!("eof-in-doctype parse error");
                if let Some(HtmlToken::Doctype { force_quirks, .. }) = &mut self.current_token {
                    *force_quirks = true;
                }
                self.emit_tokens(vec![self.current_token.clone().unwrap(), HtmlToken::Eof]);
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#after-doctype-name-state
    fn tokenize_after_doctype_name(&mut self) {
        match self.input.next() {
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' => {}
                '>' => {
                    self.state = TokenizationState::Data;
                    self.emit_token(self.current_token.clone().unwrap());
                }
                _ => {
                    unimplemented!();
                }
            },
            None => {
                eprintln!("eof-in-doctype parse error");
                if let Some(HtmlToken::Doctype { force_quirks, .. }) = &mut self.current_token {
                    *force_quirks = true;
                }
                self.emit_tokens(vec![self.current_token.clone().unwrap(), HtmlToken::Eof]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_html_tag_pair() {
        let mut tokenizer = HtmlTokenizer::new("<html></html>");
        let actual = std::iter::from_fn(|| {
            let token = tokenizer.consume();
            if token == HtmlToken::Eof {
                None
            } else {
                Some(token)
            }
        })
        .collect::<Vec<_>>();
        let expected = vec![
            HtmlToken::StartTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::EndTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false,
            },
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn tokenize_html_with_head_and_title() {
        let mut tokenizer = HtmlTokenizer::new("<html><head><title></title></head></html>");
        let actual = std::iter::from_fn(|| {
            let token = tokenizer.consume();
            if token == HtmlToken::Eof {
                None
            } else {
                Some(token)
            }
        })
        .collect::<Vec<_>>();
        let expected = vec![
            HtmlToken::StartTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::StartTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::StartTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::EndTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::EndTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::EndTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false,
            },
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn tokenize_full_html() {
        let html = "<!DOCTYPE html><html lang=\"en\"><head><title>Test</title></head><body><div id=\'main\'><br/></div></body></html>";
        let mut tokenizer = HtmlTokenizer::new(html);
        let actual = std::iter::from_fn(|| {
            let token = tokenizer.consume();
            if token == HtmlToken::Eof {
                None
            } else {
                Some(token)
            }
        })
        .collect::<Vec<_>>();
        let expected = vec![
            HtmlToken::Doctype {
                name: "html".to_string().into(),
                public_identifier: None,
                system_identifier: None,
                force_quirks: false,
            },
            HtmlToken::StartTag {
                tag_name: "html".to_string(),
                attributes: vec![("lang".to_string(), "en".to_string())],
                self_closing: false,
            },
            HtmlToken::StartTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::StartTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::Character('T'),
            HtmlToken::Character('e'),
            HtmlToken::Character('s'),
            HtmlToken::Character('t'),
            HtmlToken::EndTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::EndTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::StartTag {
                tag_name: "body".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::StartTag {
                tag_name: "div".to_string(),
                attributes: vec![("id".to_string(), "main".to_string())],
                self_closing: false,
            },
            HtmlToken::StartTag {
                tag_name: "br".to_string(),
                attributes: vec![],
                self_closing: true,
            },
            HtmlToken::EndTag {
                tag_name: "div".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::EndTag {
                tag_name: "body".to_string(),
                attributes: vec![],
                self_closing: false,
            },
            HtmlToken::EndTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false,
            },
        ];
        assert_eq!(actual, expected);
    }
}
