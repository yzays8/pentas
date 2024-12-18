use std::vec;

use anyhow::{ensure, Ok, Result};

use crate::utils::TokenIterator;

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
    OpenCurlyBrace,
    CloseCurlyBrace,

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
    input: TokenIterator<char>,
}

impl CssTokenizer {
    pub fn new(css: &str) -> Self {
        let input = TokenIterator::new(&css.chars().collect::<Vec<char>>());
        Self { input }
    }

    /// https://www.w3.org/TR/css-syntax-3/#tokenization
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

    /// https://www.w3.org/TR/css-syntax-3/#consume-token
    fn consume_token(&mut self) -> Result<CssToken> {
        self.consume_comments()?;
        match self.input.next() {
            Some(c) => match c {
                c if Self::is_whitespace(c) => {
                    while self.input.peek().is_some_and(|c| Self::is_whitespace(*c)) {
                        self.input.next();
                    }
                    Ok(CssToken::Whitespace)
                }
                '"' => Ok(self.consume_string_token(*self.input.get_last_consumed().unwrap())),
                '#' => {
                    if self.input.peek().is_some_and(|c| Self::is_ident_char(*c))
                        || Self::is_valid_escape(&[
                            self.input.get_last_consumed(),
                            self.input.peek(),
                        ])
                    {
                        let type_flag = if Self::starts_ident(&self.input.peek_chunk(3)) {
                            HashType::Id
                        } else {
                            HashType::Unrestricted
                        };
                        Ok(CssToken::Hash(self.consume_ident_sequence(), type_flag))
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                '\'' => Ok(self.consume_string_token(*self.input.get_last_consumed().unwrap())),
                '(' => Ok(CssToken::OpenParenthesis),
                ')' => Ok(CssToken::CloseParenthesis),
                '+' => {
                    if Self::starts_number(
                        &[
                            vec![self.input.get_last_consumed()],
                            self.input.peek_chunk(2),
                        ]
                        .concat(),
                    ) {
                        self.input.rewind(1);
                        Ok(self.consume_numeric_token())
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                ',' => Ok(CssToken::Comma),
                '-' => {
                    if Self::starts_number(
                        &[
                            vec![self.input.get_last_consumed()],
                            self.input.peek_chunk(2),
                        ]
                        .concat(),
                    ) {
                        self.input.rewind(1);
                        Ok(self.consume_numeric_token())
                    } else if self.input.peek_chunk(2) == [Some(&'-'), Some(&'>')] {
                        self.input.next();
                        self.input.next();
                        Ok(CssToken::Cdc)
                    } else if Self::starts_ident(
                        &[
                            vec![self.input.get_last_consumed()],
                            self.input.peek_chunk(2),
                        ]
                        .concat(),
                    ) {
                        self.input.rewind(1);
                        Ok(self.consume_ident_like_sequence())
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                '.' => {
                    if Self::starts_ident(
                        &[
                            vec![self.input.get_last_consumed()],
                            self.input.peek_chunk(2),
                        ]
                        .concat(),
                    ) {
                        self.input.rewind(1);
                        Ok(self.consume_numeric_token())
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                ':' => Ok(CssToken::Colon),
                ';' => Ok(CssToken::Semicolon),
                '<' => {
                    if self.input.peek_chunk(3) == [Some(&'!'), Some(&'-'), Some(&'-')] {
                        self.input.next();
                        self.input.next();
                        self.input.next();
                        Ok(CssToken::Cdo)
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                '@' => {
                    if Self::starts_ident(&self.input.peek_chunk(3)) {
                        Ok(CssToken::AtKeyword(self.consume_ident_sequence()))
                    } else {
                        Ok(CssToken::Delim(c))
                    }
                }
                '[' => Ok(CssToken::OpenSquareBracket),
                '\\' => {
                    if Self::is_valid_escape(&[self.input.get_last_consumed(), self.input.peek()]) {
                        self.input.rewind(1);
                        Ok(self.consume_ident_like_sequence())
                    } else {
                        eprintln!("parse error: invalid escape in consume_token");
                        Ok(CssToken::Delim(c))
                    }
                }
                ']' => Ok(CssToken::CloseSquareBracket),
                '{' => Ok(CssToken::OpenCurlyBrace),
                '}' => Ok(CssToken::CloseCurlyBrace),
                '0'..='9' => {
                    self.input.rewind(1);
                    Ok(self.consume_numeric_token())
                }
                c if Self::is_ident_start_char(c) => {
                    self.input.rewind(1);
                    Ok(self.consume_ident_like_sequence())
                }
                _ => Ok(CssToken::Delim(c)),
            },
            None => Ok(CssToken::Eof),
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-string-token
    fn consume_string_token(&mut self, ending_char: char) -> CssToken {
        let mut string = String::new();

        loop {
            match self.input.next() {
                Some(c) => match c {
                    c if c == ending_char => {
                        return CssToken::String(string);
                    }
                    '\n' => {
                        eprintln!("parse error: newline in consume_string_token");
                        self.input.rewind(1);
                        return CssToken::BadString;
                    }
                    '\\' => {
                        if self.input.peek().is_some() {
                            if *self.input.peek().unwrap() == '\n' {
                                self.input.next();
                            } else {
                                string.push(self.consume_escaped_char());
                            }
                        }
                    }
                    _ => {
                        string.push(*self.input.get_last_consumed().unwrap());
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

        if Self::starts_ident(&self.input.peek_chunk(3)) {
            CssToken::Dimension(number, self.consume_ident_sequence())
        } else if self.input.peek() == Some(&'%') {
            self.input.next();
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
            if self.input.peek_chunk(2) != [Some(&'/'), Some(&'*')] {
                break;
            }
            end_with_eof = false;
            let mut consumed_asterisk = false;
            loop {
                let c = self.input.next();
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
        if string.eq_ignore_ascii_case("url") && self.input.peek() == Some(&'(') {
            self.input.next();
            while self
                .input
                .peek_chunk(2)
                .iter()
                .all(|c| c.is_some_and(|c| Self::is_whitespace(*c)))
            {
                self.input.next();
            }
            match self.input.peek_chunk(2)[..] {
                [Some(&'"' | &'\''), _] | [Some(&' '), Some(&'"' | &'\'')] => {
                    CssToken::Function(string)
                }
                _ => self.consume_url_token(),
            }
        } else if self.input.peek() == Some(&'(') {
            self.input.next();
            CssToken::Function(string)
        } else {
            CssToken::Ident(string)
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-url-token
    fn consume_url_token(&mut self) -> CssToken {
        let mut url = String::new();
        while self.input.peek().is_some_and(|c| Self::is_whitespace(*c)) {
            self.input.next();
        }
        loop {
            match self.input.next() {
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
                    if Self::is_valid_escape(&[self.input.get_last_consumed(), self.input.peek()]) {
                        url.push(self.consume_escaped_char());
                    } else {
                        eprintln!("parse error: invalid escape in consume_url_token");
                        self.consume_remnants_of_bad_url();
                        return CssToken::BadUrl;
                    }
                }
                c if c.is_some_and(Self::is_whitespace) => {
                    while self.input.peek().is_some_and(|c| Self::is_whitespace(*c)) {
                        self.input.next();
                    }
                    if self.input.peek().is_none() {
                        eprintln!("parse error: EOF in consume_url_token");
                    }
                    if let Some(')') | None = self.input.peek() {
                        self.input.next();
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
                    url.push(*self.input.get_last_consumed().unwrap());
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-name
    fn consume_ident_sequence(&mut self) -> String {
        let mut result = String::new();
        loop {
            let c = self.input.next();
            match c {
                Some(c) if Self::is_ident_char(c) => {
                    result.push(c);
                }
                _ => {
                    if Self::is_valid_escape(&[self.input.get_last_consumed(), self.input.peek()]) {
                        result.push(self.consume_escaped_char());
                    } else {
                        self.input.rewind(1);
                        return result;
                    }
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-escaped-code-point
    fn consume_escaped_char(&mut self) -> char {
        match self.input.next() {
            Some(c) if c.is_ascii_hexdigit() => {
                unimplemented!()
            }
            None => {
                eprintln!("parse error: EOF in consume_escaped_char");
                '\u{FFFD}'
            }
            _ => *self.input.get_last_consumed().unwrap(),
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-number
    fn consume_number(&mut self) -> NumericType {
        let mut repr = String::new();
        let mut type_flag = TypeFlag::Integer;

        if let Some('+' | '-') = self.input.peek() {
            repr.push(self.input.next().unwrap());
        }

        while let Some('0'..='9') = self.input.peek() {
            repr.push(self.input.next().unwrap());
        }

        if let [Some('.'), Some('0'..='9')] = self.input.peek_chunk(2)[..] {
            repr.push(self.input.next().unwrap());
            repr.push(self.input.next().unwrap());
            while let Some('0'..='9') = self.input.peek() {
                repr.push(self.input.next().unwrap());
            }
            type_flag = TypeFlag::Number;
        }

        if let [Some('E' | 'e'), Some('0'..='9')] = self.input.peek_chunk(2)[..] {
            repr.push(self.input.next().unwrap());
            repr.push(self.input.next().unwrap());
            while let Some('0'..='9') = self.input.peek() {
                repr.push(self.input.next().unwrap());
            }
            type_flag = TypeFlag::Number;
        } else if let [Some('E' | 'e'), Some('+' | '-'), Some('0'..='9')] =
            self.input.peek_chunk(3)[..]
        {
            repr.push(self.input.next().unwrap());
            repr.push(self.input.next().unwrap());
            repr.push(self.input.next().unwrap());
            while let Some('0'..='9') = self.input.peek() {
                repr.push(self.input.next().unwrap());
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
            match self.input.next() {
                Some(')') | None => return,
                _ if Self::is_valid_escape(&[
                    self.input.get_last_consumed(),
                    self.input.peek(),
                ]) =>
                {
                    self.consume_escaped_char();
                }
                Some(_) => {}
            }
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

    /// Checks if two code points are a valid escape.
    /// https://www.w3.org/TR/css-syntax-3/#check-if-two-code-points-are-a-valid-escape
    fn is_valid_escape(chars: &[Option<&char>]) -> bool {
        if chars[0] != Some(&'\\') {
            false
        } else {
            chars[1] != Some(&'\n')
        }
    }

    /// Checks if three code points would start an ident sequence.
    /// https://www.w3.org/TR/css-syntax-3/#would-start-an-identifier
    fn starts_ident(chars: &[Option<&char>]) -> bool {
        match (chars[0], chars[1], chars[2]) {
            (Some('-'), Some('-'), _) => true,
            (Some('-'), Some(c), _) if Self::is_ident_start_char(*c) => true,
            (Some('-'), c1, c2) if Self::is_valid_escape(&[c1, c2]) => true,
            (Some('\\'), c, _) if Self::is_valid_escape(&[Some(&'\\'), c]) => true,
            (Some(c), _, _) if Self::is_ident_start_char(*c) => true,
            _ => false,
        }
    }

    /// Checks if three code points would start a number.
    /// https://www.w3.org/TR/css-syntax-3/#starts-with-a-number
    fn starts_number(chars: &[Option<&char>]) -> bool {
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
    fn consume_valid_comment() {
        let css = "/* hello, world! */";
        assert_eq!(
            CssTokenizer::new(css).tokenize().unwrap(),
            vec![CssToken::Eof]
        );

        let css = "/* hello, world! *//* Hello, World! */";
        assert_eq!(
            CssTokenizer::new(css).tokenize().unwrap(),
            vec![CssToken::Eof]
        );
    }

    #[test]
    #[should_panic]
    fn consume_invalid_comment() {
        let css = "/* hello, world!";
        assert_eq!(
            CssTokenizer::new(css).tokenize().unwrap(),
            vec![CssToken::Eof]
        );
    }

    #[test]
    fn tokenize_number_with_whitespace() {
        let css = "12345 67890";
        assert_eq!(
            CssTokenizer::new(css).tokenize().unwrap(),
            vec![
                CssToken::Number(NumericType::Integer(12345)),
                CssToken::Whitespace,
                CssToken::Number(NumericType::Integer(67890)),
                CssToken::Eof
            ]
        );
    }

    #[test]
    fn tokenize_hash() {
        let css = "#12345";
        assert_eq!(
            CssTokenizer::new(css).tokenize().unwrap(),
            vec![
                CssToken::Hash("12345".to_string(), HashType::Unrestricted),
                CssToken::Eof
            ]
        )
    }

    #[test]
    fn tokenize_number_with_dot() {
        let css = "12345.67890";
        assert_eq!(
            CssTokenizer::new(css).tokenize().unwrap(),
            vec![
                CssToken::Number(NumericType::Number(12345.67890)),
                CssToken::Eof
            ]
        );
    }

    #[test]
    fn tokenize_simple_style() {
        // https://developer.mozilla.org/en-US/docs/Learn/CSS/First_steps/Styling_a_biography_page
        let css = r#"h1 {
            color: #375e97;
            font-size: 2em;
            font-family: Georgia, 'Times New Roman', Times, serif;
            border-bottom: 1px solid #375e97;
          }"#;

        assert_eq!(
            CssTokenizer::new(css).tokenize().unwrap(),
            vec![
                CssToken::Ident("h1".to_string()),
                CssToken::Whitespace,
                CssToken::OpenCurlyBrace,
                CssToken::Whitespace,
                CssToken::Ident("color".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::Hash("375e97".to_string(), HashType::Unrestricted),
                CssToken::Semicolon,
                CssToken::Whitespace,
                CssToken::Ident("font-size".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::Dimension(NumericType::Integer(2), "em".to_string()),
                CssToken::Semicolon,
                CssToken::Whitespace,
                CssToken::Ident("font-family".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::Ident("Georgia".to_string()),
                CssToken::Comma,
                CssToken::Whitespace,
                CssToken::String("Times New Roman".to_string()),
                CssToken::Comma,
                CssToken::Whitespace,
                CssToken::Ident("Times".to_string()),
                CssToken::Comma,
                CssToken::Whitespace,
                CssToken::Ident("serif".to_string()),
                CssToken::Semicolon,
                CssToken::Whitespace,
                CssToken::Ident("border-bottom".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::Dimension(NumericType::Integer(1), "px".to_string()),
                CssToken::Whitespace,
                CssToken::Ident("solid".to_string()),
                CssToken::Whitespace,
                CssToken::Hash("375e97".to_string(), HashType::Unrestricted),
                CssToken::Semicolon,
                CssToken::Whitespace,
                CssToken::CloseCurlyBrace,
                CssToken::Eof,
            ]
        )
    }

    #[test]
    fn tokenize_simple_style_with_complex_selector() {
        let css = r#"a[href^="https"][href$=".org"] {
            color: green;
        }"#;

        assert_eq!(
            CssTokenizer::new(css).tokenize().unwrap(),
            vec![
                CssToken::Ident("a".to_string()),
                CssToken::OpenSquareBracket,
                CssToken::Ident("href".to_string()),
                CssToken::Delim('^'),
                CssToken::Delim('='),
                CssToken::String("https".to_string()),
                CssToken::CloseSquareBracket,
                CssToken::OpenSquareBracket,
                CssToken::Ident("href".to_string()),
                CssToken::Delim('$'),
                CssToken::Delim('='),
                CssToken::String(".org".to_string()),
                CssToken::CloseSquareBracket,
                CssToken::Whitespace,
                CssToken::OpenCurlyBrace,
                CssToken::Whitespace,
                CssToken::Ident("color".to_string()),
                CssToken::Colon,
                CssToken::Whitespace,
                CssToken::Ident("green".to_string()),
                CssToken::Semicolon,
                CssToken::Whitespace,
                CssToken::CloseCurlyBrace,
                CssToken::Eof,
            ]
        )
    }
}
