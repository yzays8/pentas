use std::collections::VecDeque;
use std::default::Default;

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
pub struct Tokenizer {
    state: TokenizationState,
    input: Vec<char>,
    current_pos: usize,
    current_token: Token,
    output: VecDeque<Token>,
}

/// The output of the tokenization step is a series of zero or more of the following tokens:
/// DOCTYPE, start tag, end tag, comment, character, end-of-file.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Token {
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
    #[default]
    Eof,
}

impl Tokenizer {
    pub fn new(input: String) -> Self {
        Self {
            state: TokenizationState::Data,
            input: input.chars().collect(),
            current_pos: 0,
            current_token: Default::default(),
            output: VecDeque::new(),
        }
    }

    pub fn consume_token(&mut self) -> Token {
        // When a token is emitted, it must immediately be handled by the tree construction stage.
        if self.output.is_empty() {
            loop {
                match self.state {
                    // https://html.spec.whatwg.org/multipage/parsing.html#data-state
                    TokenizationState::Data => match self.consume_char() {
                        Some(c) => match c {
                            '&' => {
                                unimplemented!();
                            }
                            '<' => {
                                self.state = TokenizationState::TagOpen;
                            }
                            '\u{0000}' => {
                                eprintln!("unexpected-null-character parse error");
                                self.output.push_back(Token::Character('\u{FFFD}'));
                                break;
                            }
                            _ => {
                                self.output.push_back(Token::Character(c));
                                break;
                            }
                        },
                        None => {
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
                    TokenizationState::TagOpen => match self.consume_char() {
                        Some(c) => match c {
                            '!' => {
                                self.state = TokenizationState::MarkupDeclarationOpen;
                            }
                            '/' => {
                                self.state = TokenizationState::EndTagOpen;
                            }
                            c if c.is_alphabetic() => {
                                self.current_token = Token::StartTag {
                                    tag_name: String::new(),
                                    attributes: vec![],
                                    self_closing: false,
                                };
                                self.reconsume(TokenizationState::TagName);
                            }
                            _ => {
                                eprintln!("invalid-first-character-of-tag-name parse error");
                                self.reconsume(TokenizationState::Data);
                                self.output.push_back(Token::Character('<'));
                                break;
                            }
                        },
                        None => {
                            eprintln!("eof-before-tag-name parse error");
                            self.output.push_back(Token::Character('<'));
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#end-tag-open-state
                    TokenizationState::EndTagOpen => match self.consume_char() {
                        Some(c) => match c {
                            c if c.is_alphabetic() => {
                                self.current_token = Token::EndTag {
                                    tag_name: String::new(),
                                    attributes: vec![],
                                    self_closing: false,
                                };
                                self.reconsume(TokenizationState::TagName);
                            }
                            '>' => {
                                eprintln!("missing-end-tag-name parse error");
                                self.state = TokenizationState::Data;
                            }
                            _ => {
                                eprintln!("invalid-first-character-of-tag-name parse error");
                                self.current_token = Token::Comment(String::new());
                                self.reconsume(TokenizationState::BogusComment);
                            }
                        },
                        None => {
                            eprintln!("eof-before-tag-name parse error");
                            self.output.push_back(Token::Character('<'));
                            self.output.push_back(Token::Character('/'));
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#tag-name-state
                    TokenizationState::TagName => match self.consume_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                self.state = TokenizationState::BeforeAttributeName;
                            }
                            '/' => {
                                self.state = TokenizationState::SelfClosingStartTag;
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone());
                                break;
                            }
                            c if c.is_ascii_uppercase() => {
                                if let Token::StartTag { tag_name, .. }
                                | Token::EndTag { tag_name, .. } = &mut self.current_token
                                {
                                    tag_name.push(c.to_ascii_lowercase());
                                }
                            }
                            '\u{0000}' => {
                                if let Token::StartTag { tag_name, .. }
                                | Token::EndTag { tag_name, .. } = &mut self.current_token
                                {
                                    eprintln!("unexpected-null-character parse error");
                                    tag_name.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if let Token::StartTag { tag_name, .. }
                                | Token::EndTag { tag_name, .. } = &mut self.current_token
                                {
                                    tag_name.push(c);
                                }
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
                    TokenizationState::BeforeAttributeName => match self.consume_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                continue;
                            }
                            '/' | '>' => {
                                self.reconsume(TokenizationState::AfterAttributeName);
                            }
                            '=' => {
                                eprintln!(
                                    "unexpected-equals-sign-before-attribute-name parse error"
                                );

                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    attributes.push((c.to_string(), String::new()));
                                }
                            }
                            _ => {
                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    attributes.push((String::new(), String::new()));
                                }
                                self.reconsume(TokenizationState::AttributeName);
                            }
                        },
                        None => {
                            self.reconsume(TokenizationState::AfterAttributeName);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#attribute-name-state
                    TokenizationState::AttributeName => match self.consume_char() {
                        // todo: UA needs to check if current attribute's name is already in the attributes list before leaving this state
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' | '/' | '>' => {
                                self.reconsume(TokenizationState::AfterAttributeName);
                            }
                            '=' => {
                                self.state = TokenizationState::BeforeAttributeValue;
                            }
                            c if c.is_ascii_uppercase() => match &mut self.current_token {
                                Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } => {
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
                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    eprintln!("unexpected-null-character parse error");
                                    attributes.last_mut().unwrap().0.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if c == '"' || c == '\'' || c == '<' {
                                    eprintln!("invalid-character-in-attribute-name parse error");
                                }
                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    attributes.last_mut().unwrap().0.push(c);
                                }
                            }
                        },
                        None => {
                            self.reconsume(TokenizationState::AfterAttributeName);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-name-state
                    TokenizationState::AfterAttributeName => match self.consume_char() {
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
                                self.output.push_back(self.current_token.clone());
                                break;
                            }
                            _ => {
                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    attributes.push((String::new(), String::new()));
                                }
                                self.reconsume(TokenizationState::AttributeName);
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-value-state
                    TokenizationState::BeforeAttributeValue => match self.consume_char() {
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
                                self.output.push_back(self.current_token.clone());
                                break;
                            }
                            _ => {
                                self.reconsume(TokenizationState::AttributeValueUnquoted);
                            }
                        },
                        _ => {
                            self.reconsume(TokenizationState::AttributeValueUnquoted);
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(double-quoted)-state
                    TokenizationState::AttributeValueDoubleQuoted => match self.consume_char() {
                        Some(c) => match c {
                            '"' => {
                                self.state = TokenizationState::AfterAttributeValueQuoted;
                            }
                            '&' => {
                                unimplemented!();
                            }
                            '\u{0000}' => {
                                eprintln!("unexpected-null-character parse error");
                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    attributes.last_mut().unwrap().1.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    attributes.last_mut().unwrap().1.push(c);
                                }
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(single-quoted)-state
                    TokenizationState::AttributeValueSingleQuoted => match self.consume_char() {
                        Some(c) => match c {
                            '\'' => {
                                self.state = TokenizationState::AfterAttributeValueQuoted;
                            }
                            '&' => {
                                unimplemented!();
                            }
                            '\u{0000}' => {
                                eprintln!("unexpected-null-character parse error");
                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    attributes.last_mut().unwrap().1.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    attributes.last_mut().unwrap().1.push(c);
                                }
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(unquoted)-state
                    TokenizationState::AttributeValueUnquoted => match self.consume_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                self.state = TokenizationState::BeforeAttributeName;
                            }
                            '&' => {
                                unimplemented!();
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone());
                                break;
                            }
                            '\u{0000}' => {
                                eprintln!("unexpected-null-character parse error");
                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    attributes.last_mut().unwrap().1.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if c == '"' || c == '\'' || c == '<' || c == '=' || c == '`' {
                                    eprintln!("unexpected-character-in-unquoted-attribute-value parse error");
                                }
                                if let Token::StartTag { attributes, .. }
                                | Token::EndTag { attributes, .. } = &mut self.current_token
                                {
                                    attributes.last_mut().unwrap().1.push(c);
                                }
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-value-(quoted)-state
                    TokenizationState::AfterAttributeValueQuoted => match self.consume_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                self.state = TokenizationState::BeforeAttributeName;
                            }
                            '/' => {
                                self.state = TokenizationState::SelfClosingStartTag;
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone());
                                break;
                            }
                            _ => {
                                eprintln!("missing-whitespace-between-attributes parse error");
                                self.reconsume(TokenizationState::BeforeAttributeName);
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#self-closing-start-tag-state
                    TokenizationState::SelfClosingStartTag => match self.consume_char() {
                        Some(c) => match c {
                            '>' => {
                                if let Token::StartTag { self_closing, .. }
                                | Token::EndTag { self_closing, .. } = &mut self.current_token
                                {
                                    *self_closing = true;
                                }
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone());
                                break;
                            }
                            _ => {
                                eprintln!("unexpected-solidus-in-tag parse error");
                                self.reconsume(TokenizationState::BeforeAttributeName);
                            }
                        },
                        None => {
                            eprintln!("eof-in-tag parse error");
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#bogus-comment-state
                    TokenizationState::BogusComment => match self.consume_char() {
                        Some(c) => match c {
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone());
                                break;
                            }
                            '\u{0000}' => {
                                eprintln!("unexpected-null-character parse error");
                                if let Token::Comment(comment) = &mut self.current_token {
                                    comment.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if let Token::Comment(comment) = &mut self.current_token {
                                    comment.push(c);
                                }
                            }
                        },
                        None => {
                            self.output.push_back(self.current_token.clone());
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#markup-declaration-open-state
                    TokenizationState::MarkupDeclarationOpen => {
                        if self.peek_str(2) == "--" {
                            unimplemented!();
                        } else if self.peek_str(7).to_uppercase() == "DOCTYPE" {
                            self.current_pos += 7;
                            self.state = TokenizationState::Doctype;
                        } else if self.peek_str(7) == "[CDATA[" {
                            unimplemented!();
                        } else {
                            eprintln!("incorrectly-opened-comment parse error");
                            self.current_token = Token::Comment(String::new());
                            self.state = TokenizationState::BogusComment;
                        }
                    }

                    // https://html.spec.whatwg.org/multipage/parsing.html#doctype-state
                    TokenizationState::Doctype => match self.consume_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                self.state = TokenizationState::BeforeDoctypeName;
                            }
                            '>' => {
                                self.reconsume(TokenizationState::BeforeDoctypeName);
                            }
                            _ => {
                                eprintln!("missing-whitespace-before-doctype-name parse error");
                                self.reconsume(TokenizationState::BeforeDoctypeName);
                            }
                        },
                        None => {
                            eprintln!("eof-in-doctype parse error");
                            self.current_token = Token::Doctype {
                                name: None,
                                public_identifier: None,
                                system_identifier: None,
                                force_quirks: true,
                            };
                            self.output.push_back(self.current_token.clone());
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#before-doctype-name-state
                    TokenizationState::BeforeDoctypeName => match self.consume_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                continue;
                            }
                            c if c.is_ascii_uppercase() => {
                                self.current_token = Token::Doctype {
                                    name: c.to_ascii_lowercase().to_string().into(),
                                    public_identifier: None,
                                    system_identifier: None,
                                    force_quirks: false,
                                };
                                self.state = TokenizationState::DoctypeName;
                            }
                            '\u{0000}' => {
                                eprintln!("unexpected-null-character parse error");
                                self.current_token = Token::Doctype {
                                    name: '\u{FFFD}'.to_string().into(),
                                    public_identifier: None,
                                    system_identifier: None,
                                    force_quirks: false,
                                };
                                self.state = TokenizationState::DoctypeName;
                            }
                            '>' => {
                                eprintln!("missing-doctype-name parse error");
                                self.current_token = Token::Doctype {
                                    name: None,
                                    public_identifier: None,
                                    system_identifier: None,
                                    force_quirks: true,
                                };
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone());
                                break;
                            }
                            _ => {
                                self.current_token = Token::Doctype {
                                    name: c.to_string().into(),
                                    public_identifier: None,
                                    system_identifier: None,
                                    force_quirks: false,
                                };
                                self.state = TokenizationState::DoctypeName;
                            }
                        },
                        None => {
                            eprintln!("eof-in-doctype parse error");
                            self.current_token = Token::Doctype {
                                name: None,
                                public_identifier: None,
                                system_identifier: None,
                                force_quirks: true,
                            };
                            self.output.push_back(self.current_token.clone());
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#doctype-name-state
                    TokenizationState::DoctypeName => match self.consume_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                self.state = TokenizationState::AfterDoctypeName;
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone());
                                break;
                            }
                            c if c.is_ascii_uppercase() => match &mut self.current_token {
                                Token::Doctype { name, .. } => {
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
                                if let Token::Doctype { name: Some(n), .. } =
                                    &mut self.current_token
                                {
                                    n.push('\u{FFFD}');
                                }
                            }
                            _ => {
                                if let Token::Doctype { name: Some(n), .. } =
                                    &mut self.current_token
                                {
                                    n.push(c);
                                }
                            }
                        },
                        None => {
                            eprintln!("eof-in-doctype parse error");
                            if let Token::Doctype { force_quirks, .. } = &mut self.current_token {
                                *force_quirks = true;
                            }
                            self.output.push_back(self.current_token.clone());
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },

                    // https://html.spec.whatwg.org/multipage/parsing.html#after-doctype-name-state
                    TokenizationState::AfterDoctypeName => match self.consume_char() {
                        Some(c) => match c {
                            '\t' | '\n' | '\x0C' | ' ' => {
                                continue;
                            }
                            '>' => {
                                self.state = TokenizationState::Data;
                                self.output.push_back(self.current_token.clone());
                                break;
                            }
                            _ => {
                                unimplemented!();
                            }
                        },
                        None => {
                            eprintln!("eof-in-doctype parse error");
                            if let Token::Doctype { force_quirks, .. } = &mut self.current_token {
                                *force_quirks = true;
                            }
                            self.output.push_back(self.current_token.clone());
                            self.output.push_back(Token::Eof);
                            break;
                        }
                    },
                }
            }
        }

        assert!(!self.output.is_empty());
        self.output.pop_front().unwrap()
    }

    /// When a state says to reconsume a matched character in a specified state, that means to switch to that state,
    /// but when it attempts to consume the next input character, provide it with the current input character instead.
    fn reconsume(&mut self, move_to: TokenizationState) {
        self.state = move_to;
        self.current_pos -= 1;
    }

    fn consume_char(&mut self) -> Option<char> {
        self.input.get(self.current_pos).map(|c| {
            self.current_pos += 1;
            *c
        })
    }

    fn peek_str(&self, len: usize) -> String {
        self.input.iter().skip(self.current_pos).take(len).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_char() {
        let mut tokenizer = Tokenizer::new("he llo, world!\n".to_string());
        assert_eq!(tokenizer.consume_char(), Some('h'));
        assert_eq!(tokenizer.consume_char(), Some('e'));
        assert_eq!(tokenizer.consume_char(), Some(' '));
        assert_eq!(tokenizer.consume_char(), Some('l'));
        assert_eq!(tokenizer.consume_char(), Some('l'));
        assert_eq!(tokenizer.consume_char(), Some('o'));
        assert_eq!(tokenizer.consume_char(), Some(','));
        tokenizer.current_pos -= 1;
        assert_eq!(tokenizer.consume_char(), Some(','));
        assert_eq!(tokenizer.consume_char(), Some(' '));
        assert_eq!(tokenizer.consume_char(), Some('w'));
        assert_eq!(tokenizer.consume_char(), Some('o'));
        assert_eq!(tokenizer.consume_char(), Some('r'));
        assert_eq!(tokenizer.consume_char(), Some('l'));
        tokenizer.current_pos -= 1;
        assert_eq!(tokenizer.consume_char(), Some('l'));
        assert_eq!(tokenizer.consume_char(), Some('d'));
        assert_eq!(tokenizer.consume_char(), Some('!'));
        assert_eq!(tokenizer.consume_char(), Some('\n'));
        assert_eq!(tokenizer.consume_char(), None);
    }

    #[test]
    fn test_peek_str() {
        let tokenizer = Tokenizer::new("hello".to_string());
        assert_eq!(tokenizer.peek_str(5), "hello");
        assert_eq!(tokenizer.peek_str(3), "hel");
    }

    #[test]
    fn test_consume_token1() {
        let mut tokenizer = Tokenizer::new("<html></html>".to_string());
        assert_eq!(
            tokenizer.consume_token(),
            Token::StartTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::EndTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(tokenizer.consume_token(), Token::Eof);
    }

    #[test]
    fn test_consume_token2() {
        let mut tokenizer = Tokenizer::new("<html><head><title></title></head></html>".to_string());
        assert_eq!(
            tokenizer.consume_token(),
            Token::StartTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::StartTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::StartTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::EndTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::EndTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::EndTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(tokenizer.consume_token(), Token::Eof);
    }

    #[test]
    fn test_consume_token3() {
        let html = "<!DOCTYPE html><html lang=\"en\"><head><title>Test</title></head><body><div id=\'main\'><br/></div></body></html>";
        let mut tokenizer = Tokenizer::new(html.to_string());
        assert_eq!(
            tokenizer.consume_token(),
            Token::Doctype {
                name: "html".to_string().into(),
                public_identifier: None,
                system_identifier: None,
                force_quirks: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::StartTag {
                tag_name: "html".to_string(),
                attributes: vec![("lang".to_string(), "en".to_string())],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::StartTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::StartTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(tokenizer.consume_token(), Token::Character('T'));
        assert_eq!(tokenizer.consume_token(), Token::Character('e'));
        assert_eq!(tokenizer.consume_token(), Token::Character('s'));
        assert_eq!(tokenizer.consume_token(), Token::Character('t'));
        assert_eq!(
            tokenizer.consume_token(),
            Token::EndTag {
                tag_name: "title".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::EndTag {
                tag_name: "head".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::StartTag {
                tag_name: "body".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::StartTag {
                tag_name: "div".to_string(),
                attributes: vec![("id".to_string(), "main".to_string())],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::StartTag {
                tag_name: "br".to_string(),
                attributes: vec![],
                self_closing: true
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::EndTag {
                tag_name: "div".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::EndTag {
                tag_name: "body".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(
            tokenizer.consume_token(),
            Token::EndTag {
                tag_name: "html".to_string(),
                attributes: vec![],
                self_closing: false
            }
        );
        assert_eq!(tokenizer.consume_token(), Token::Eof);
    }
}
