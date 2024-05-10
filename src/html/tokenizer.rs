use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq)]
enum TokenizationState {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
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
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
}

#[derive(Debug)]
pub struct HtmlTokenizer {
    state: TokenizationState,
    input: Vec<char>,
    current_pos: usize,
    current_token: Option<HtmlToken>,
    output: VecDeque<HtmlToken>,
}

/// The output of the tokenization step is a series of zero or more of the following tokens:
/// DOCTYPE, start tag, end tag, comment, character, end-of-file.
#[derive(Debug, Clone, PartialEq)]
pub enum HtmlToken {
    // When a DOCTYPE token is created, its name, public identifier, and system identifier
    // must be marked as missing (which is a distinct state from the empty string),
    // and the force-quirks flag must be set to off (its other state is on).
    Doctype {
        name: Option<String>,
        public_identifier: Option<String>,
        system_identifier: Option<String>,
        force_quirks: bool,
    },

    // When a start or end tag token is created, its self-closing flag must be
    // unset, and its attributes list must be empty.
    StartTag {
        tag_name: String,
        attributes: Vec<(String, String)>, // (name, value)
        self_closing: bool,
    },
    EndTag {
        tag_name: String,
        attributes: Vec<(String, String)>, // (name, value)
        self_closing: bool,
    },

    Comment(String),
    Character(char),
    Eof,
}

impl HtmlTokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            state: TokenizationState::Data,
            input: input.chars().collect(),
            current_pos: 0,
            current_token: None,
            output: VecDeque::new(),
        }
    }

    /// When a state says to reconsume a matched character in a specified state, that means to switch to that state,
    /// but when it attempts to consume the next input character, provide it with the current input character instead.
    fn allow_reconsume(&mut self, move_to: TokenizationState) {
        self.state = move_to;
        self.current_pos -= 1;
    }

    fn consume_input_char(&mut self) -> Option<char> {
        self.input.get(self.current_pos).map(|c| {
            self.current_pos += 1;
            *c
        })
    }

    fn peek_input_str(&self, len: usize) -> String {
        self.input.iter().skip(self.current_pos).take(len).collect()
    }

    pub fn consume_token(&mut self) -> HtmlToken {
        // When a token is emitted, it must immediately be handled by the tree construction stage.
        if self.output.is_empty() {
            loop {
                match self.state {
                    // https://html.spec.whatwg.org/multipage/parsing.html#data-state
                    TokenizationState::Data => match self.consume_input_char() {
                        Some(c) => match c {
                            '&' => {
                                unimplemented!();
                            }
                            '<' => {
                                self.state = TokenizationState::TagOpen;
                            }
                            '\u{0000}' => {
                                eprintln!("unexpected-null-character parse error");
                                self.output.push_back(HtmlToken::Character('\u{FFFD}'));
                                break;
                            }
                            _ => {
                                self.output.push_back(HtmlToken::Character(c));
                                break;
                            }
                        },
                        None => {
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
                    TokenizationState::TagOpen => match self.consume_input_char() {
                        Some(c) => match c {
                            '!' => {
                                self.state = TokenizationState::MarkupDeclarationOpen;
                            }
                            '/' => {
                                self.state = TokenizationState::EndTagOpen;
                            }
                            c if c.is_alphabetic() => {
                                self.current_token = Some(HtmlToken::StartTag {
                                    tag_name: String::new(),
                                    attributes: vec![],
                                    self_closing: false,
                                });
                                self.allow_reconsume(TokenizationState::TagName);
                            }
                            _ => {
                                eprintln!("invalid-first-character-of-tag-name parse error");
                                self.allow_reconsume(TokenizationState::Data);
                                self.output.push_back(HtmlToken::Character('<'));
                                break;
                            }
                        },
                        None => {
                            eprintln!("eof-before-tag-name parse error");
                            self.output.push_back(HtmlToken::Character('<'));
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#end-tag-open-state
                    TokenizationState::EndTagOpen => match self.consume_input_char() {
                        Some(c) => match c {
                            c if c.is_alphabetic() => {
                                self.current_token = Some(HtmlToken::EndTag {
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
                                self.current_token = Some(HtmlToken::Comment(String::new()));
                                self.allow_reconsume(TokenizationState::BogusComment);
                            }
                        },
                        None => {
                            eprintln!("eof-before-tag-name parse error");
                            self.output.push_back(HtmlToken::Character('<'));
                            self.output.push_back(HtmlToken::Character('/'));
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#tag-name-state
                    TokenizationState::TagName => match self.consume_input_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                self.state = TokenizationState::BeforeAttributeName;
                            }
                            '/' => {
                                self.state = TokenizationState::SelfClosingStartTag;
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone().unwrap());
                                break;
                            }
                            c if c.is_ascii_uppercase() => {
                                if let Some(HtmlToken::StartTag { tag_name, .. })
                                | Some(HtmlToken::EndTag { tag_name, .. }) =
                                    &mut self.current_token
                                {
                                    tag_name.push(c.to_ascii_lowercase());
                                }
                            }
                            '\u{0000}' => {
                                if let Some(HtmlToken::StartTag { tag_name, .. })
                                | Some(HtmlToken::EndTag { tag_name, .. }) =
                                    &mut self.current_token
                                {
                                    eprintln!("unexpected-null-character parse error");
                                    tag_name.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if let Some(HtmlToken::StartTag { tag_name, .. })
                                | Some(HtmlToken::EndTag { tag_name, .. }) =
                                    &mut self.current_token
                                {
                                    tag_name.push(c);
                                }
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
                    TokenizationState::BeforeAttributeName => match self.consume_input_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                continue;
                            }
                            '/' | '>' => {
                                self.allow_reconsume(TokenizationState::AfterAttributeName);
                            }
                            '=' => {
                                eprintln!(
                                    "unexpected-equals-sign-before-attribute-name parse error"
                                );

                                if let Some(HtmlToken::StartTag { attributes, .. })
                                | Some(HtmlToken::EndTag { attributes, .. }) =
                                    &mut self.current_token
                                {
                                    attributes.push((c.to_string(), String::new()));
                                }
                            }
                            _ => {
                                if let Some(HtmlToken::StartTag { attributes, .. })
                                | Some(HtmlToken::EndTag { attributes, .. }) =
                                    &mut self.current_token
                                {
                                    attributes.push((String::new(), String::new()));
                                }
                                self.allow_reconsume(TokenizationState::AttributeName);
                            }
                        },
                        None => {
                            self.allow_reconsume(TokenizationState::AfterAttributeName);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#attribute-name-state
                    TokenizationState::AttributeName => match self.consume_input_char() {
                        // todo: UA needs to check if current attribute's name is already in the attributes list before leaving this state
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' | '/' | '>' => {
                                self.allow_reconsume(TokenizationState::AfterAttributeName);
                            }
                            '=' => {
                                self.state = TokenizationState::BeforeAttributeValue;
                            }
                            c if c.is_ascii_uppercase() => match &mut self.current_token {
                                Some(HtmlToken::StartTag { attributes, .. })
                                | Some(HtmlToken::EndTag { attributes, .. }) => {
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
                                if let Some(HtmlToken::StartTag { attributes, .. })
                                | Some(HtmlToken::EndTag { attributes, .. }) =
                                    &mut self.current_token
                                {
                                    eprintln!("unexpected-null-character parse error");
                                    attributes.last_mut().unwrap().0.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if c == '"' || c == '\'' || c == '<' {
                                    eprintln!("invalid-character-in-attribute-name parse error");
                                }
                                if let Some(HtmlToken::StartTag { attributes, .. })
                                | Some(HtmlToken::EndTag { attributes, .. }) =
                                    &mut self.current_token
                                {
                                    attributes.last_mut().unwrap().0.push(c);
                                }
                            }
                        },
                        None => {
                            self.allow_reconsume(TokenizationState::AfterAttributeName);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-name-state
                    TokenizationState::AfterAttributeName => match self.consume_input_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                continue;
                            }
                            '/' => {
                                self.state = TokenizationState::SelfClosingStartTag;
                            }
                            '=' => {
                                self.state = TokenizationState::BeforeAttributeValue;
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone().unwrap());
                                break;
                            }
                            _ => {
                                if let Some(HtmlToken::StartTag { attributes, .. })
                                | Some(HtmlToken::EndTag { attributes, .. }) =
                                    &mut self.current_token
                                {
                                    attributes.push((String::new(), String::new()));
                                }
                                self.allow_reconsume(TokenizationState::AttributeName);
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-value-state
                    TokenizationState::BeforeAttributeValue => match self.consume_input_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                continue;
                            }
                            '"' => {
                                self.state = TokenizationState::AttributeValueDoubleQuoted;
                            }
                            '\'' => {
                                self.state = TokenizationState::AttributeValueSingleQuoted;
                            }
                            '>' => {
                                eprintln!("missing-attribute-value parse error");
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone().unwrap());
                                break;
                            }
                            _ => {
                                self.allow_reconsume(TokenizationState::AttributeValueUnquoted);
                            }
                        },
                        _ => {
                            self.allow_reconsume(TokenizationState::AttributeValueUnquoted);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(double-quoted)-state
                    TokenizationState::AttributeValueDoubleQuoted => {
                        match self.consume_input_char() {
                            Some(c) => match c {
                                '"' => {
                                    self.state = TokenizationState::AfterAttributeValueQuoted;
                                }
                                '&' => {
                                    unimplemented!();
                                }
                                '\u{0000}' => {
                                    eprintln!("unexpected-null-character parse error");
                                    if let Some(HtmlToken::StartTag { attributes, .. })
                                    | Some(HtmlToken::EndTag { attributes, .. }) =
                                        &mut self.current_token
                                    {
                                        attributes.last_mut().unwrap().1.push('\u{FFFD}');
                                    }
                                }
                                _ => {
                                    if let Some(HtmlToken::StartTag { attributes, .. })
                                    | Some(HtmlToken::EndTag { attributes, .. }) =
                                        &mut self.current_token
                                    {
                                        attributes.last_mut().unwrap().1.push(c);
                                    }
                                }
                            },
                            None => {
                                eprintln!("eof-in-tag parse error");
                                self.output.push_back(HtmlToken::Eof);
                                break;
                            }
                        }
                    }

                    // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(single-quoted)-state
                    TokenizationState::AttributeValueSingleQuoted => {
                        match self.consume_input_char() {
                            Some(c) => match c {
                                '\'' => {
                                    self.state = TokenizationState::AfterAttributeValueQuoted;
                                }
                                '&' => {
                                    unimplemented!();
                                }
                                '\u{0000}' => {
                                    eprintln!("unexpected-null-character parse error");
                                    if let Some(HtmlToken::StartTag { attributes, .. })
                                    | Some(HtmlToken::EndTag { attributes, .. }) =
                                        &mut self.current_token
                                    {
                                        attributes.last_mut().unwrap().1.push('\u{FFFD}');
                                    }
                                }
                                _ => {
                                    if let Some(HtmlToken::StartTag { attributes, .. })
                                    | Some(HtmlToken::EndTag { attributes, .. }) =
                                        &mut self.current_token
                                    {
                                        attributes.last_mut().unwrap().1.push(c);
                                    }
                                }
                            },
                            None => {
                                eprintln!("eof-in-tag parse error");
                                self.output.push_back(HtmlToken::Eof);
                                break;
                            }
                        }
                    }

                    // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(unquoted)-state
                    TokenizationState::AttributeValueUnquoted => match self.consume_input_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                self.state = TokenizationState::BeforeAttributeName;
                            }
                            '&' => {
                                unimplemented!();
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone().unwrap());
                                break;
                            }
                            '\u{0000}' => {
                                eprintln!("unexpected-null-character parse error");
                                if let Some(HtmlToken::StartTag { attributes, .. })
                                | Some(HtmlToken::EndTag { attributes, .. }) =
                                    &mut self.current_token
                                {
                                    attributes.last_mut().unwrap().1.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if c == '"' || c == '\'' || c == '<' || c == '=' || c == '`' {
                                    eprintln!("unexpected-character-in-unquoted-attribute-value parse error");
                                }
                                if let Some(HtmlToken::StartTag { attributes, .. })
                                | Some(HtmlToken::EndTag { attributes, .. }) =
                                    &mut self.current_token
                                {
                                    attributes.last_mut().unwrap().1.push(c);
                                }
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-value-(quoted)-state
                    TokenizationState::AfterAttributeValueQuoted => match self.consume_input_char()
                    {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                self.state = TokenizationState::BeforeAttributeName;
                            }
                            '/' => {
                                self.state = TokenizationState::SelfClosingStartTag;
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone().unwrap());
                                break;
                            }
                            _ => {
                                eprintln!("missing-whitespace-between-attributes parse error");
                                self.allow_reconsume(TokenizationState::BeforeAttributeName);
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#self-closing-start-tag-state
                    TokenizationState::SelfClosingStartTag => match self.consume_input_char() {
                        Some(c) => match c {
                            '>' => {
                                if let Some(HtmlToken::StartTag { self_closing, .. })
                                | Some(HtmlToken::EndTag { self_closing, .. }) =
                                    &mut self.current_token
                                {
                                    *self_closing = true;
                                }
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone().unwrap());
                                break;
                            }
                            _ => {
                                eprintln!("unexpected-solidus-in-tag parse error");
                                self.allow_reconsume(TokenizationState::BeforeAttributeName);
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#bogus-comment-state
                    TokenizationState::BogusComment => match self.consume_input_char() {
                        Some(c) => match c {
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone().unwrap());
                                break;
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
                            self.output.push_back(self.current_token.clone().unwrap());
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#markup-declaration-open-state
                    TokenizationState::MarkupDeclarationOpen => {
                        if self.peek_input_str(2) == "--" {
                            unimplemented!();
                        } else if self.peek_input_str(7).to_uppercase() == "DOCTYPE" {
                            self.current_pos += 7;
                            self.state = TokenizationState::Doctype;
                        } else if self.peek_input_str(7) == "[CDATA[" {
                            unimplemented!();
                        } else {
                            eprintln!("incorrectly-opened-comment parse error");
                            self.current_token = Some(HtmlToken::Comment(String::new()));
                            self.state = TokenizationState::BogusComment;
                        }
                    }

                    // https://html.spec.whatwg.org/multipage/parsing.html#doctype-state
                    TokenizationState::Doctype => match self.consume_input_char() {
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
                            self.current_token = Some(HtmlToken::Doctype {
                                name: None,
                                public_identifier: None,
                                system_identifier: None,
                                force_quirks: true,
                            });
                            self.output.push_back(self.current_token.clone().unwrap());
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#before-doctype-name-state
                    TokenizationState::BeforeDoctypeName => match self.consume_input_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                continue;
                            }
                            c if c.is_ascii_uppercase() => {
                                self.current_token = Some(HtmlToken::Doctype {
                                    name: c.to_ascii_lowercase().to_string().into(),
                                    public_identifier: None,
                                    system_identifier: None,
                                    force_quirks: false,
                                });
                                self.state = TokenizationState::DoctypeName;
                            }
                            '\u{0000}' => {
                                eprintln!("unexpected-null-character parse error");
                                self.current_token = Some(HtmlToken::Doctype {
                                    name: '\u{FFFD}'.to_string().into(),
                                    public_identifier: None,
                                    system_identifier: None,
                                    force_quirks: false,
                                });
                                self.state = TokenizationState::DoctypeName;
                            }
                            '>' => {
                                eprintln!("missing-doctype-name parse error");
                                self.current_token = Some(HtmlToken::Doctype {
                                    name: None,
                                    public_identifier: None,
                                    system_identifier: None,
                                    force_quirks: true,
                                });
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone().unwrap());
                                break;
                            }
                            _ => {
                                self.current_token = Some(HtmlToken::Doctype {
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
                            self.current_token = Some(HtmlToken::Doctype {
                                name: None,
                                public_identifier: None,
                                system_identifier: None,
                                force_quirks: true,
                            });
                            self.output.push_back(self.current_token.clone().unwrap());
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#doctype-name-state
                    TokenizationState::DoctypeName => match self.consume_input_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                self.state = TokenizationState::AfterDoctypeName;
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone().unwrap());
                                break;
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
                                if let Some(HtmlToken::Doctype { name: Some(n), .. }) =
                                    &mut self.current_token
                                {
                                    n.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if let Some(HtmlToken::Doctype { name: Some(n), .. }) =
                                    &mut self.current_token
                                {
                                    n.push(c);
                                }
                            }
                        },
                        None => {
                            eprintln!("eof-in-doctype parse error");
                            if let Some(HtmlToken::Doctype { force_quirks, .. }) =
                                &mut self.current_token
                            {
                                *force_quirks = true;
                            }
                            self.output.push_back(self.current_token.clone().unwrap());
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#after-doctype-name-state
                    TokenizationState::AfterDoctypeName => match self.consume_input_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                continue;
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone().unwrap());
                                break;
                            }
                            _ => {
                                unimplemented!();
                            }
                        },
                        None => {
                            eprintln!("eof-in-doctype parse error");
                            if let Some(HtmlToken::Doctype { force_quirks, .. }) =
                                &mut self.current_token
                            {
                                *force_quirks = true;
                            }
                            self.output.push_back(self.current_token.clone().unwrap());
                            self.output.push_back(HtmlToken::Eof);
                            break;
                        }
                    },
                }
            }
        }

        assert!(!self.output.is_empty());
        self.output.pop_front().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_char() {
        let mut tokenizer = HtmlTokenizer::new("he llo, world!\n");
        assert_eq!(tokenizer.consume_input_char(), Some('h'));
        assert_eq!(tokenizer.consume_input_char(), Some('e'));
        assert_eq!(tokenizer.consume_input_char(), Some(' '));
        assert_eq!(tokenizer.consume_input_char(), Some('l'));
        assert_eq!(tokenizer.consume_input_char(), Some('l'));
        assert_eq!(tokenizer.consume_input_char(), Some('o'));
        assert_eq!(tokenizer.consume_input_char(), Some(','));
        tokenizer.current_pos -= 1;
        assert_eq!(tokenizer.consume_input_char(), Some(','));
        assert_eq!(tokenizer.consume_input_char(), Some(' '));
        assert_eq!(tokenizer.consume_input_char(), Some('w'));
        assert_eq!(tokenizer.consume_input_char(), Some('o'));
        assert_eq!(tokenizer.consume_input_char(), Some('r'));
        assert_eq!(tokenizer.consume_input_char(), Some('l'));
        tokenizer.current_pos -= 1;
        assert_eq!(tokenizer.consume_input_char(), Some('l'));
        assert_eq!(tokenizer.consume_input_char(), Some('d'));
        assert_eq!(tokenizer.consume_input_char(), Some('!'));
        assert_eq!(tokenizer.consume_input_char(), Some('\n'));
        assert_eq!(tokenizer.consume_input_char(), None);
    }

    #[test]
    fn test_peek_input_str() {
        let tokenizer = HtmlTokenizer::new("hello");
        assert_eq!(tokenizer.peek_input_str(5), "hello");
        assert_eq!(tokenizer.peek_input_str(3), "hel");
    }

    #[test]
    fn test_consume_token1() {
        let mut tokenizer = HtmlTokenizer::new("<html></html>");
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::StartTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::EndTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(tokenizer.consume_token(), HtmlToken::Eof);
    }

    #[test]
    fn test_consume_token2() {
        let mut tokenizer = HtmlTokenizer::new("<html><head><title></title></head></html>");
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::StartTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::StartTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::StartTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::EndTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::EndTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::EndTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(tokenizer.consume_token(), HtmlToken::Eof);
    }

    #[test]
    fn test_consume_token3() {
        let html = "<!DOCTYPE html><html lang=\"en\"><head><title>Test</title></head><body><div id=\'main\'><br/></div></body></html>";
        let mut tokenizer = HtmlTokenizer::new(html);
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::Doctype {
                name: "html".to_string().into(),
                public_identifier: None,
                system_identifier: None,
                force_quirks: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::StartTag {
                tag_name: "html".to_string(),
                attributes: vec![("lang".to_string(), "en".to_string())],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::StartTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::StartTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(tokenizer.consume_token(), HtmlToken::Character('T'));
        assert_eq!(tokenizer.consume_token(), HtmlToken::Character('e'));
        assert_eq!(tokenizer.consume_token(), HtmlToken::Character('s'));
        assert_eq!(tokenizer.consume_token(), HtmlToken::Character('t'));
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::EndTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::EndTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::StartTag {
                tag_name: "body".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::StartTag {
                tag_name: "div".to_string(),
                attributes: vec![("id".to_string(), "main".to_string())],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::StartTag {
                tag_name: "br".to_string(),
                attributes: vec![],
                self_closing: true
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::EndTag {
                tag_name: "div".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::EndTag {
                tag_name: "body".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            HtmlToken::EndTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(tokenizer.consume_token(), HtmlToken::Eof);
    }
}
