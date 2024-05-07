use std::vec;

use anyhow::{bail, Ok, Result};

/// https://www.w3.org/TR/css-syntax-3/#tokenization
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Ident(String),
    Function(String),
    AtKeyword(String),
    Hash(String, HashType),
    String(String),
    BadString(String),
    Url(String),
    BadUrl(String),
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
    // EOF is a special token that is used to indicate the end of the input stream.
    Eof,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum HashType {
    Id,
    #[default]
    Unrestricted,
}

#[derive(Debug, Default, PartialEq)]
pub enum TypeFlag {
    #[default]
    Integer,
    Number,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NumericType {
    Integer(i32),
    Number(f32),
}

#[derive(Debug)]
pub struct Tokenizer {
    input: Vec<char>,
    current_pos: usize,

    // The last code point to have been consumed
    current_char: Option<char>,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            current_pos: 0,
            current_char: None,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let token = self.consume_token()?;
            tokens.push(token.clone());
            if token == Token::Eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-token
    pub fn consume_token(&mut self) -> Result<Token> {
        self.consume_comments()?;
        match self.consume_char() {
            Some(c) => match c {
                '\n' | '\t' | ' ' => {
                    while let Some('\n') | Some('\t') | Some(' ') = self.peek_char() {
                        self.consume_char();
                    }
                    Ok(Token::Whitespace)
                }
                '"' => Ok(self.consume_string_token(None)?),
                '#' => {
                    if (self.peek_char().is_some()
                        && Tokenizer::is_ident_code_point(self.peek_char().unwrap()))
                        || self.are_valid_escape(Some(self.peek_str(2)))
                    {
                        let type_flag = if self.would_start_ident(Some(self.peek_str(3))) {
                            HashType::Id
                        } else {
                            HashType::Unrestricted
                        };
                        Ok(Token::Hash(self.consume_ident_sequence(), type_flag))
                    } else {
                        Ok(Token::Delim(c))
                    }
                }
                '\'' => Ok(self.consume_string_token(None)?),
                '(' => Ok(Token::OpenParenthesis),
                ')' => Ok(Token::CloseParenthesis),
                '+' => {
                    if self.start_with_number(None) {
                        self.current_pos -= 1;
                        Ok(self.consume_numeric_token())
                    } else {
                        Ok(Token::Delim(c))
                    }
                }
                ',' => Ok(Token::Comma),
                '-' => {
                    if self.start_with_number(None) {
                        self.current_pos -= 1;
                        Ok(self.consume_numeric_token())
                    } else if self.peek_str(2) == vec![Some('-'), Some('>')] {
                        self.consume_char();
                        self.consume_char();
                        Ok(Token::Cdc)
                    } else if self.would_start_ident(None) {
                        self.current_pos -= 1;
                        Ok(self.consume_ident_like_sequence())
                    } else {
                        Ok(Token::Delim(c))
                    }
                }
                '.' => {
                    if self.would_start_ident(None) {
                        self.current_pos -= 1;
                        Ok(self.consume_numeric_token())
                    } else {
                        Ok(Token::Delim(c))
                    }
                }
                ':' => Ok(Token::Colon),
                ';' => Ok(Token::Semicolon),
                '<' => {
                    if self.peek_str(3) == vec![Some('!'), Some('-'), Some('-')] {
                        self.consume_char();
                        self.consume_char();
                        self.consume_char();
                        Ok(Token::Cdo)
                    } else {
                        Ok(Token::Delim(c))
                    }
                }
                '@' => {
                    if self.would_start_ident(Some(self.peek_str(3))) {
                        Ok(Token::AtKeyword(self.consume_ident_sequence()))
                    } else {
                        Ok(Token::Delim(c))
                    }
                }
                '[' => Ok(Token::OpenSquareBracket),
                '\\' => {
                    if self.are_valid_escape(None) {
                        self.current_pos -= 1;
                        Ok(self.consume_ident_like_sequence())
                    } else {
                        eprintln!("parse error: invalid escape in consume_token");
                        Ok(Token::Delim(c))
                    }
                }
                ']' => Ok(Token::CloseSquareBracket),
                '{' => Ok(Token::OpenBrace),
                '}' => Ok(Token::CloseBrace),
                '0'..='9' => {
                    self.current_pos -= 1;
                    Ok(self.consume_numeric_token())
                }
                c if Tokenizer::is_ident_start_char(c) => {
                    self.current_pos -= 1;
                    Ok(self.consume_ident_like_sequence())
                }
                _ => Ok(Token::Delim(c)),
            },
            None => Ok(Token::Eof),
        }
    }

    fn consume_char(&mut self) -> Option<char> {
        let c = self.input.get(self.current_pos).copied();
        self.current_char = c;
        self.current_pos += 1;
        c
    }

    fn peek_char(&self) -> Option<char> {
        self.input.get(self.current_pos).copied()
    }

    fn peek_str(&self, len: usize) -> Vec<Option<char>> {
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

    fn is_ident_start_char(c: char) -> bool {
        c.is_ascii_alphabetic() || c >= '\u{0080}' || c == '_'
    }

    fn is_ident_code_point(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '-' || c == '_' || c >= '\u{0080}'
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-string-token
    fn consume_string_token(&mut self, ending_char: Option<char>) -> Result<Token> {
        let mut string = String::new();
        let ending_char = if let Some(ending_char) = ending_char {
            ending_char
        } else {
            if let Some(c) = self.current_char {
                c
            } else {
                bail!("current input code point does not exist");
            }
        };

        loop {
            match self.consume_char() {
                Some(c) => match c {
                    c if c == ending_char => {
                        return Ok(Token::String(string));
                    }
                    '\n' => {
                        eprintln!("parse error: newline in consume_string_token");
                        unimplemented!();
                    }
                    '\\' => {
                        if self.peek_char().is_some() {
                            if self.peek_char().unwrap() == '\n' {
                                self.consume_char();
                            } else {
                                unimplemented!();
                            }
                        }
                    }
                    _ => {
                        string.push(self.current_char.unwrap());
                    }
                },
                None => {
                    eprintln!("parse error: EOF in consume_string_token");
                    return Ok(Token::String(string));
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-numeric-token
    fn consume_numeric_token(&mut self) -> Token {
        let number = self.consume_number();

        if self.would_start_ident(Some(self.peek_str(3))) {
            Token::Dimension(number, self.consume_ident_sequence())
        } else if self.peek_char() == Some('%') {
            self.consume_char();
            Token::Percentage(match number {
                NumericType::Integer(i) => i as f32,
                NumericType::Number(f) => f,
            })
        } else {
            Token::Number(number)
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-comment
    fn consume_comments(&mut self) -> Result<()> {
        let mut end_with_eof = false;

        loop {
            if self.peek_str(2) != vec![Some('/'), Some('*')] {
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
        if end_with_eof {
            bail!("parse error: consuming comments ended with EOF");
        }
        Ok(())
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-ident-like-token
    fn consume_ident_like_sequence(&mut self) -> Token {
        let string = self.consume_ident_sequence();
        if string.eq_ignore_ascii_case("url") && self.peek_char() == Some('(') {
            self.consume_char();
            unimplemented!();
        } else if self.peek_char() == Some('(') {
            self.consume_char();
            Token::Function(string)
        } else {
            Token::Ident(string)
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-name
    fn consume_ident_sequence(&mut self) -> String {
        let mut result = String::new();
        loop {
            let c = self.consume_char();
            match c {
                Some(c) if Tokenizer::is_ident_code_point(c) => {
                    result.push(c);
                }
                _ => {
                    if self.are_valid_escape(None) {
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

        if let Some('+') | Some('-') = self.peek_char() {
            repr.push(self.consume_char().unwrap());
        }

        while let Some('0'..='9') = self.peek_char() {
            repr.push(self.consume_char().unwrap());
        }

        let two_chars = self.peek_str(2);
        if two_chars[0] == Some('.')
            && two_chars[1].is_some()
            && two_chars[1].unwrap().is_ascii_digit()
        {
            repr.push(self.consume_char().unwrap());
            repr.push(self.consume_char().unwrap());
            while let Some('0'..='9') = self.peek_char() {
                repr.push(self.consume_char().unwrap());
            }
            type_flag = TypeFlag::Number;
        }

        let three_chars = self.peek_str(3);
        if (three_chars[0] == Some('E') || three_chars[0] == Some('e'))
            && (three_chars[1].is_some() && three_chars[1].unwrap().is_ascii_digit())
        {
            repr.push(self.consume_char().unwrap());
            repr.push(self.consume_char().unwrap());
            while let Some('0'..='9') = self.peek_char() {
                repr.push(self.consume_char().unwrap());
            }
            type_flag = TypeFlag::Number;
        } else if (three_chars[0] == Some('E') || three_chars[0] == Some('e'))
            && (three_chars[1] == Some('+') || three_chars[1] == Some('-'))
            && (three_chars[2].is_some() && three_chars[2].unwrap().is_ascii_digit())
        {
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

    /// https://www.w3.org/TR/css-syntax-3/#check-if-two-code-points-are-a-valid-escape
    fn are_valid_escape(&self, chars: Option<Vec<Option<char>>>) -> bool {
        let two_chars = if let Some(chars) = chars {
            chars
        } else {
            let mut ret = vec![self.current_char];
            ret.extend(self.peek_str(2));
            ret
        };

        if two_chars[0] != Some('\\') {
            false
        } else {
            two_chars[1] != Some('\n')
        }
    }

    /// Check if three code points would start an ident sequence
    /// https://www.w3.org/TR/css-syntax-3/#would-start-an-identifier
    fn would_start_ident(&self, chars: Option<Vec<Option<char>>>) -> bool {
        let three_chars = if let Some(chars) = chars {
            chars
        } else {
            let mut ret = vec![self.current_char];
            ret.extend(self.peek_str(2));
            ret
        };
        match three_chars[0] {
            Some('-') => {
                ((three_chars[1].is_some()
                    && Tokenizer::is_ident_start_char(three_chars[1].unwrap()))
                    || three_chars[1] == Some('-'))
                    || self.are_valid_escape(Some(vec![three_chars[1], three_chars[2]]))
            }
            c if c.is_some() && Tokenizer::is_ident_start_char(c.unwrap()) => true,
            Some('\\') => three_chars[1] != Some('\n'),
            _ => false,
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#starts-with-a-number
    fn start_with_number(&self, chars: Option<Vec<Option<char>>>) -> bool {
        let three_chars = if let Some(chars) = chars {
            chars
        } else {
            let mut ret = vec![self.current_char];
            ret.extend(self.peek_str(2));
            ret
        };
        match three_chars[0] {
            Some(c1) => match c1 {
                '+' | '-' => {
                    if let Some(c2) = three_chars[1] {
                        match c2 {
                            c2 if c2.is_ascii_digit() => true,
                            '.' => {
                                if let Some(c3) = three_chars[2] {
                                    c3.is_ascii_digit()
                                } else {
                                    false
                                }
                            }
                            _ => false,
                        }
                    } else {
                        false
                    }
                }
                '.' => {
                    if let Some(c2) = three_chars[1] {
                        c2.is_ascii_digit()
                    } else {
                        false
                    }
                }
                '0'..='9' => true,
                _ => false,
            },
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_char() {
        let mut tokenizer = Tokenizer::new("he llo, world!\n");
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
        let mut tokenizer = Tokenizer::new("hi");
        assert_eq!(tokenizer.peek_str(2), vec![Some('h'), Some('i')]);
        assert_eq!(tokenizer.peek_str(3), vec![Some('h'), Some('i'), None]);
        assert_eq!(
            tokenizer.peek_str(4),
            vec![Some('h'), Some('i'), None, None]
        );
    }

    #[test]
    fn test_consume_comments() {
        let mut tokenizer = Tokenizer::new("/* hello, world! */");
        assert!(tokenizer.consume_comments().is_ok());
        assert_eq!(tokenizer.consume_char(), None);

        let mut tokenizer = Tokenizer::new("/* hello, world!");
        assert!(tokenizer.consume_comments().is_err());
        assert_eq!(tokenizer.consume_char(), None);

        let mut tokenizer = Tokenizer::new(r"/* hello, world! *//* Hello, World! */");
        assert!(tokenizer.consume_comments().is_ok());
        assert_eq!(tokenizer.consume_char(), None);
    }

    #[test]
    fn test_consume_token1() {
        let mut tokenizer = Tokenizer::new("12345 67890");
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Number(NumericType::Integer(12345))
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Number(NumericType::Integer(67890))
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Eof);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_consume_token2() {
        let mut tokenizer = Tokenizer::new("#12345");
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Hash("12345".to_string(), HashType::Unrestricted)
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Eof);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_consume_token3() {
        let mut tokenizer = Tokenizer::new("12345.67890");
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Number(NumericType::Number(12345.67890))
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Eof);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_consume_token4() {
        // From: https://developer.mozilla.org/en-US/docs/Learn/CSS/First_steps/Styling_a_biography_page
        let mut tokenizer = Tokenizer::new(
            r"h1 {
            color: #375e97;
            font-size: 2em;
            font-family: Georgia, 'Times New Roman', Times, serif;
            border-bottom: 1px solid #375e97;
          }",
        );
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Ident("h1".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);

        assert_eq!(tokenizer.consume_token().unwrap(), Token::OpenBrace);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);

        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Ident("color".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Colon);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Hash("375e97".to_string(), HashType::Unrestricted)
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Semicolon);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);

        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Ident("font-size".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Colon);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Dimension(NumericType::Integer(2), "em".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Semicolon);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);

        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Ident("font-family".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Colon);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Ident("Georgia".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Comma);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::String("Times New Roman".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Comma);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Ident("Times".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Comma);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Ident("serif".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Semicolon);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);

        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Ident("border-bottom".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Colon);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Dimension(NumericType::Integer(1), "px".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Ident("solid".to_string())
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);
        assert_eq!(
            tokenizer.consume_token().unwrap(),
            Token::Hash("375e97".to_string(), HashType::Unrestricted)
        );
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Semicolon);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Whitespace);

        assert_eq!(tokenizer.consume_token().unwrap(), Token::CloseBrace);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Eof);
        assert_eq!(tokenizer.consume_token().unwrap(), Token::Eof);
    }
}
