use std::vec;

use anyhow::{ensure, Ok, Result};

/// https://www.w3.org/TR/css-syntax-3/#tokenization
#[derive(Clone, Debug, PartialEq)]
pub enum CssToken {
    Ident(String),
    Function(String),
    AtKeyword(String),
    Hash(String, HashType),
    String(String),
    BadString,
    Url(String),
    BadUrl,
    Delim(char),
    Number(NumericType),
    Percentage(f32),
    Dimension(NumericType, String),
    Whitespace,
    Cdo,
    Cdc,
    Colon,
    Semicolon,
    Comma,
    OpenSquareBracket,
    CloseSquareBracket,
    OpenParenthesis,
    CloseParenthesis,
    OpenBrace,
    CloseBrace,

    /// EOF is a special token that is used to indicate the end of the input stream.
    Eof,
}

/// The default hash type is unrestricted.
#[derive(Clone, Debug, PartialEq)]
pub enum HashType {
    Id,
    Unrestricted,
}

/// The default type flag is integer.
#[derive(Debug, PartialEq)]
pub enum TypeFlag {
    Integer,
    Number,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NumericType {
    Integer(i32),
    Number(f32),
}

#[derive(Debug)]
pub struct CssTokenizer {
    input: Vec<char>,
    current_pos: usize,

    // The last code point to have been consumed
    current_char: Option<char>,
}

impl CssTokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            current_pos: 0,
            current_char: None,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<CssToken>> {
        let mut tokens = Vec::new();
        loop {
            let token = self.consume_token()?;
            tokens.push(token.clone());
            if token == CssToken::Eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// Returns the next character from the input stream and advances the current position.
    fn consume_char(&mut self) -> Option<char> {
        let c = self.input.get(self.current_pos).copied();
        self.current_char = c;
        self.current_pos += 1;
        c
    }

    /// Returns the next character from the input stream without consuming it.
    fn peek_char(&self) -> Option<char> {
        self.input.get(self.current_pos).copied()
    }

    /// Returns the next `len` characters from the input stream without consuming them.
    fn peek_chars(&self, len: usize) -> Vec<Option<char>> {
        let mut iter = self
            .input
            .iter()
            .skip(self.current_pos)
            .take(len)
            .map(|c| Some(*c))
            .collect::<Vec<Option<char>>>();
        for _ in iter.len()..len {
            iter.push(None);
        }
        assert_eq!(iter.len(), len);
        iter
    }

    /// Returns the next `len` characters from the input stream, including the current character, without consuming them.
    fn peek_chars_with_curr_char(&self, len: usize) -> Vec<Option<char>> {
        let mut ret = vec![self.current_char];
        ret.extend(self.peek_chars(len - 1));
        ret
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-token
    fn consume_token(&mut self) -> Result<CssToken> {
        self.consume_comments()?;
        match self.consume_char() {
            Some(c) => match c {
                c if Self::is_whitespace(c) => {
                    while self.peek_char().is_some_and(Self::is_whitespace) {
                        self.consume_char();
                    }
                    Ok(CssToken::Whitespace)
                }
                '"' => Ok(self.consume_string_token(self.current_char.unwrap())),
                '#' => {
                    if self.peek_char().is_some_and(Self::is_ident_char)
                        || Self::are_valid_escape(&self.peek_chars_with_curr_char(2))
                    {
                        let type_flag = if Self::would_start_ident(&self.peek_chars(3)) {
                            HashType::Id
                        } else {
                            HashType::Unrestricted
                        };
                        Ok(CssToken::Hash(self.consume_ident_sequence(), type_flag))
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                '\'' => Ok(self.consume_string_token(self.current_char.unwrap())),
                '(' => Ok(CssToken::OpenParenthesis),
                ')' => Ok(CssToken::CloseParenthesis),
                '+' => {
                    if Self::starts_with_number(&self.peek_chars_with_curr_char(3)) {
                        self.current_pos -= 1;
                        Ok(self.consume_numeric_token())
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                ',' => Ok(CssToken::Comma),
                '-' => {
                    if Self::starts_with_number(&self.peek_chars_with_curr_char(3)) {
                        self.current_pos -= 1;
                        Ok(self.consume_numeric_token())
                    } else if self.peek_chars(2)[..] == [Some('-'), Some('>')] {
                        self.consume_char();
                        self.consume_char();
                        Ok(CssToken::Cdc)
                    } else if Self::would_start_ident(&self.peek_chars_with_curr_char(3)) {
                        self.current_pos -= 1;
                        Ok(self.consume_ident_like_sequence())
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                '.' => {
                    if Self::would_start_ident(&self.peek_chars_with_curr_char(3)) {
                        self.current_pos -= 1;
                        Ok(self.consume_numeric_token())
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                ':' => Ok(CssToken::Colon),
                ';' => Ok(CssToken::Semicolon),
                '<' => {
                    if self.peek_chars(3)[..] == [Some('!'), Some('-'), Some('-')] {
                        self.consume_char();
                        self.consume_char();
                        self.consume_char();
                        Ok(CssToken::Cdo)
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                '@' => {
                    if Self::would_start_ident(&self.peek_chars(3)) {
                        Ok(CssToken::AtKeyword(self.consume_ident_sequence()))
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                '[' => Ok(CssToken::OpenSquareBracket),
                '\\' => {
                    if Self::are_valid_escape(&self.peek_chars_with_curr_char(2)) {
                        self.current_pos -= 1;
                        Ok(self.consume_ident_like_sequence())
                    } else {
                        eprintln!("parse error: invalid escape in consume_token");
                        Ok(CssToken::Delim(c))
                    }
                }
                ']' => Ok(CssToken::CloseSquareBracket),
                '{' => Ok(CssToken::OpenBrace),
                '}' => Ok(CssToken::CloseBrace),
                '0'..='9' => {
                    self.current_pos -= 1;
                    Ok(self.consume_numeric_token())
                }
                c if Self::is_ident_start_char(c) => {
                    self.current_pos -= 1;
                    Ok(self.consume_ident_like_sequence())
                }
                _ => Ok(CssToken::Delim(c)),
            },
            None => Ok(CssToken::Eof),
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#ident-start-code-point
    fn is_ident_start_char(c: char) -> bool {
        c.is_ascii_alphabetic() || c >= '\u{0080}' || c == '_'
    }

    /// https://www.w3.org/TR/css-syntax-3/#ident-code-point
    fn is_ident_char(c: char) -> bool {
        Self::is_ident_start_char(c) || c.is_ascii_digit() || c == '-'
    }

    /// https://w3.org/TR/css-syntax-3/#whitespace
    fn is_whitespace(c: char) -> bool {
        matches!(c, '\n' | '\t' | ' ')
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-string-token
    fn consume_string_token(&mut self, ending_char: char) -> CssToken {
        let mut string = String::new();

        loop {
            match self.consume_char() {
                Some(c) => match c {
                    c if c == ending_char => {
                        return CssToken::String(string);
                    }
                    '\n' => {
                        eprintln!("parse error: newline in consume_string_token");
                        self.current_pos -= 1;
                        return CssToken::BadString;
                    }
                    '\\' => {
                        if self.peek_char().is_some() {
                            if self.peek_char().unwrap() == '\n' {
                                self.consume_char();
                            } else {
                                string.push(self.consume_escaped_char());
                            }
                        }
                    }
                    _ => {
                        string.push(self.current_char.unwrap());
                    }
                },
                None => {
                    eprintln!("parse error: EOF in consume_string_token");
                    return CssToken::String(string);
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-numeric-token
    fn consume_numeric_token(&mut self) -> CssToken {
        let number = self.consume_number();

        if Self::would_start_ident(&self.peek_chars(3)) {
            CssToken::Dimension(number, self.consume_ident_sequence())
        } else if self.peek_char() == Some('%') {
            self.consume_char();
            CssToken::Percentage(match number {
                NumericType::Integer(i) => i as f32,
                NumericType::Number(f) => f,
            })
        } else {
            CssToken::Number(number)
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-comment
    fn consume_comments(&mut self) -> Result<()> {
        let mut end_with_eof = false;

        loop {
            if self.peek_chars(2)[..] != [Some('/'), Some('*')] {
                break;
            }
            end_with_eof = false;
            let mut consumed_asterisk = false;
            loop {
                let c = self.consume_char();
                match c {
                    Some('*') => consumed_asterisk = true,
                    Some('/') if consumed_asterisk => break,
                    None => {
                        end_with_eof = true;
                        break;
                    }
                    _ => consumed_asterisk = false,
                }
            }
        }
        ensure!(
            !end_with_eof,
            "parse error: consuming comments ended with EOF"
        );
        Ok(())
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-ident-like-token
    fn consume_ident_like_sequence(&mut self) -> CssToken {
        let string = self.consume_ident_sequence();
        if string.eq_ignore_ascii_case("url") && self.peek_char() == Some('(') {
            self.consume_char();
            while self
                .peek_chars(2)
                .iter()
                .all(|c| c.is_some_and(Self::is_whitespace))
            {
                self.consume_char();
            }
            match self.peek_chars(2)[..] {
                [Some('"' | '\''), _] | [Some(' '), Some('"' | '\'')] => CssToken::Function(string),
                _ => self.consume_url_token(),
            }
        } else if self.peek_char() == Some('(') {
            self.consume_char();
            CssToken::Function(string)
        } else {
            CssToken::Ident(string)
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-url-token
    fn consume_url_token(&mut self) -> CssToken {
        let mut url = String::new();
        while self.peek_char().is_some_and(Self::is_whitespace) {
            self.consume_char();
        }
        loop {
            match self.consume_char() {
                Some(')') => return CssToken::Url(url),
                Some(
                    '"'
                    | '\''
                    | '('
                    | '\u{0000}'..='\u{0008}'
                    | '\u{000B}'
                    | '\u{000E}'..='\u{001F}'
                    | '\u{007F}',
                ) => {
                    eprintln!("parse error: invalid character in consume_url_token");
                    self.consume_remnants_of_bad_url();
                    return CssToken::BadUrl;
                }
                Some('\\') => {
                    if Self::are_valid_escape(&self.peek_chars_with_curr_char(2)) {
                        url.push(self.consume_escaped_char());
                    } else {
                        eprintln!("parse error: invalid escape in consume_url_token");
                        self.consume_remnants_of_bad_url();
                        return CssToken::BadUrl;
                    }
                }
                c if c.is_some_and(Self::is_whitespace) => {
                    while self.peek_char().is_some_and(Self::is_whitespace) {
                        self.consume_char();
                    }
                    if self.peek_char().is_none() {
                        eprintln!("parse error: EOF in consume_url_token");
                    }
                    if let Some(')') | None = self.peek_char() {
                        self.consume_char();
                        return CssToken::Url(url);
                    }
                    self.consume_remnants_of_bad_url();
                    return CssToken::BadUrl;
                }
                None => {
                    eprintln!("parse error: EOF in consume_url_token");
                    return CssToken::Url(url);
                }
                _ => {
                    url.push(self.current_char.unwrap());
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-name
    fn consume_ident_sequence(&mut self) -> String {
        let mut result = String::new();
        loop {
            let c = self.consume_char();
            match c {
                Some(c) if Self::is_ident_char(c) => {
                    result.push(c);
                }
                _ => {
                    if Self::are_valid_escape(&self.peek_chars_with_curr_char(2)) {
                        result.push(self.consume_escaped_char());
                    } else {
                        self.current_pos -= 1;
                        return result;
                    }
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-escaped-code-point
    fn consume_escaped_char(&mut self) -> char {
        match self.consume_char() {
            Some(c) if c.is_ascii_hexdigit() => {
                unimplemented!()
            }
            None => {
                eprintln!("parse error: EOF in consume_escaped_char");
                '\u{FFFD}'
            }
            _ => self.current_char.unwrap(),
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-number
    fn consume_number(&mut self) -> NumericType {
        let mut repr = String::new();
        let mut type_flag = TypeFlag::Integer;

        if let Some('+' | '-') = self.peek_char() {
            repr.push(self.consume_char().unwrap());
        }

        while let Some('0'..='9') = self.peek_char() {
            repr.push(self.consume_char().unwrap());
        }

        if let [Some('.'), Some('0'..='9')] = self.peek_chars(2)[..] {
            repr.push(self.consume_char().unwrap());
            repr.push(self.consume_char().unwrap());
            while let Some('0'..='9') = self.peek_char() {
                repr.push(self.consume_char().unwrap());
            }
            type_flag = TypeFlag::Number;
        }

        if let [Some('E' | 'e'), Some('0'..='9')] = self.peek_chars(2)[..] {
            repr.push(self.consume_char().unwrap());
            repr.push(self.consume_char().unwrap());
            while let Some('0'..='9') = self.peek_char() {
                repr.push(self.consume_char().unwrap());
            }
            type_flag = TypeFlag::Number;
        } else if let [Some('E' | 'e'), Some('+' | '-'), Some('0'..='9')] = self.peek_chars(3)[..] {
            repr.push(self.consume_char().unwrap());
            repr.push(self.consume_char().unwrap());
            repr.push(self.consume_char().unwrap());
            while let Some('0'..='9') = self.peek_char() {
                repr.push(self.consume_char().unwrap());
            }
            type_flag = TypeFlag::Number;
        }

        // todo: need more accurate conversion
        match type_flag {
            TypeFlag::Integer => NumericType::Integer(repr.parse().unwrap()),
            TypeFlag::Number => NumericType::Number(repr.parse().unwrap()),
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-remnants-of-bad-url
    fn consume_remnants_of_bad_url(&mut self) {
        loop {
            match self.consume_char() {
                Some(')') | None => return,
                _ if Self::are_valid_escape(&self.peek_chars_with_curr_char(2)) => {
                    self.consume_escaped_char();
                }
                Some(_) => {}
            }
        }
    }

    /// Check if two code points are a valid escape.
    /// https://www.w3.org/TR/css-syntax-3/#check-if-two-code-points-are-a-valid-escape
    fn are_valid_escape(chars: &[Option<char>]) -> bool {
        if chars[0] != Some('\\') {
            false
        } else {
            chars[1] != Some('\n')
        }
    }

    /// Check if three code points would start an ident sequence.
    /// https://www.w3.org/TR/css-syntax-3/#would-start-an-identifier
    fn would_start_ident(chars: &[Option<char>]) -> bool {
        match (chars[0], chars[1], chars[2]) {
            (Some('-'), Some('-'), _) => true,
            (Some('-'), Some(c), _) if Self::is_ident_start_char(c) => true,
            (Some('-'), c1, c2) if Self::are_valid_escape(&[c1, c2]) => true,
            (Some('\\'), c, _) if Self::are_valid_escape(&[Some('\\'), c]) => true,
            (Some(c), _, _) if Self::is_ident_start_char(c) => true,
            _ => false,
        }
    }

    /// Check if three code points would start a number.
    /// https://www.w3.org/TR/css-syntax-3/#starts-with-a-number
    fn starts_with_number(chars: &[Option<char>]) -> bool {
        matches!(
            (chars[0], chars[1], chars[2]),
            (Some('0'..='9'), _, _)
                | (Some('+' | '-' | '.'), Some('0'..='9'), _)
                | (Some('+' | '-'), Some('.'), Some('0'..='9'))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_input_char() {
        let mut tokenizer = CssTokenizer::new("he llo, world!\n");
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
    fn test_peek_input_str() {
        let tokenizer = CssTokenizer::new("hi");
        assert_eq!(tokenizer.peek_chars(2), vec![Some('h'), Some('i')]);
        assert_eq!(tokenizer.peek_chars(3), vec![Some('h'), Some('i'), None]);
        assert_eq!(
            tokenizer.peek_chars(4),
            vec![Some('h'), Some('i'), None, None]
        );
    }

    #[test]
    fn test_consume_comments() {
        let mut tokenizer = CssTokenizer::new("/* hello, world! */");
        assert!(tokenizer.consume_comments().is_ok());
        assert_eq!(tokenizer.consume_char(), None);

        let mut tokenizer = CssTokenizer::new("/* hello, world!");
        assert!(tokenizer.consume_comments().is_err());
        assert_eq!(tokenizer.consume_char(), None);

        let mut tokenizer = CssTokenizer::new(r"/* hello, world! *//* Hello, World! */");
        assert!(tokenizer.consume_comments().is_ok());
        assert_eq!(tokenizer.consume_char(), None);
    }

    #[test]
    fn test_consume_token1() {
        let mut tokenizer = CssTokenizer::new("12345 67890");
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Number(NumericType::Integer(12345))
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Number(NumericType::Integer(67890))
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Eof);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Eof);
    }

    #[test]
    fn test_consume_token2() {
        let mut tokenizer = CssTokenizer::new("#12345");
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Hash("12345".to_string(), HashType::Unrestricted)
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Eof);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Eof);
    }

    #[test]
    fn test_consume_token3() {
        let mut tokenizer = CssTokenizer::new("12345.67890");
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Number(NumericType::Number(12345.67890))
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Eof);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Eof);
    }

    #[test]
    fn test_consume_token4() {
        // From: https://developer.mozilla.org/en-US/docs/Learn/CSS/First_steps/Styling_a_biography_page
        let mut tokenizer = CssTokenizer::new(
            r"h1 {
            color: #375e97;
            font-size: 2em;
            font-family: Georgia, 'Times New Roman', Times, serif;
            border-bottom: 1px solid #375e97;
          }",
        );
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("h1".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);

        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::OpenBrace);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);

        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("color".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Colon);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Hash("375e97".to_string(), HashType::Unrestricted)
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Semicolon);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);

        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("font-size".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Colon);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Dimension(NumericType::Integer(2), "em".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Semicolon);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);

        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("font-family".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Colon);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("Georgia".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Comma);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::String("Times New Roman".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Comma);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("Times".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Comma);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("serif".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Semicolon);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);

        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("border-bottom".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Colon);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Dimension(NumericType::Integer(1), "px".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("solid".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Hash("375e97".to_string(), HashType::Unrestricted)
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Semicolon);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);

        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::CloseBrace);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Eof);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Eof);
    }

    #[test]
    fn test_consume_token5() {
        let mut tokenizer = CssTokenizer::new(
            r#"a[href^="https"][href$=".org"] {
                color: green;
            }"#,
        );
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("a".to_string())
        );
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::OpenSquareBracket
        );
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("href".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Delim('^'));
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Delim('='));
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::String("https".to_string())
        );
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::CloseSquareBracket
        );
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::OpenSquareBracket
        );
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("href".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Delim('$'));
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Delim('='));
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::String(".org".to_string())
        );
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::CloseSquareBracket
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::OpenBrace);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("color".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Colon);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            CssToken::Ident("green".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Semicolon);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Whitespace);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::CloseBrace);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Eof);
        assert_eq!(tokenizer.consume_token().unwrap(), CssToken::Eof);
    }
}
