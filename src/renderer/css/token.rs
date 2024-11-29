use std::vec;

use anyhow::{ensure, Ok, Result};

use crate::renderer::utils::TokenIterator;

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

/// https://www.w3.org/TR/css-syntax-3/#tokenization
pub fn tokenize_css(css: &str) -> Result<Vec<CssToken>> {
    let mut iter = TokenIterator::new(&css.chars().collect::<Vec<char>>());
    let mut tokens = Vec::new();
    loop {
        let token = consume_token(&mut iter)?;
        tokens.push(token.clone());
        if token == CssToken::Eof {
            break;
        }
    }
    Ok(tokens)
}

/// https://www.w3.org/TR/css-syntax-3/#consume-token
fn consume_token(chars: &mut TokenIterator<char>) -> Result<CssToken> {
    consume_comments(chars)?;
    match chars.next() {
        Some(c) => match c {
            c if is_whitespace(c) => {
                while chars.peek().is_some_and(|c| is_whitespace(*c)) {
                    chars.next();
                }
                Ok(CssToken::Whitespace)
            }
            '"' => Ok(consume_string_token(
                chars,
                *chars.get_last_consumed().unwrap(),
            )),
            '#' => {
                if chars.peek().is_some_and(|c| is_ident_char(*c))
                    || is_valid_escape(&[chars.get_last_consumed(), chars.peek()])
                {
                    let type_flag = if starts_ident(&chars.peek_chunk(3)) {
                        HashType::Id
                    } else {
                        HashType::Unrestricted
                    };
                    Ok(CssToken::Hash(consume_ident_sequence(chars), type_flag))
                } else {
                    Ok(CssToken::Delim(c))
                }
            }
            '\'' => Ok(consume_string_token(
                chars,
                *chars.get_last_consumed().unwrap(),
            )),
            '(' => Ok(CssToken::OpenParenthesis),
            ')' => Ok(CssToken::CloseParenthesis),
            '+' => {
                if starts_number(&[vec![chars.get_last_consumed()], chars.peek_chunk(2)].concat()) {
                    chars.rewind(1);
                    Ok(consume_numeric_token(chars))
                } else {
                    Ok(CssToken::Delim(c))
                }
            }
            ',' => Ok(CssToken::Comma),
            '-' => {
                if starts_number(&[vec![chars.get_last_consumed()], chars.peek_chunk(2)].concat()) {
                    chars.rewind(1);
                    Ok(consume_numeric_token(chars))
                } else if chars.peek_chunk(2) == [Some(&'-'), Some(&'>')] {
                    chars.next();
                    chars.next();
                    Ok(CssToken::Cdc)
                } else if starts_ident(
                    &[vec![chars.get_last_consumed()], chars.peek_chunk(2)].concat(),
                ) {
                    chars.rewind(1);
                    Ok(consume_ident_like_sequence(chars))
                } else {
                    Ok(CssToken::Delim(c))
                }
            }
            '.' => {
                if starts_ident(&[vec![chars.get_last_consumed()], chars.peek_chunk(2)].concat()) {
                    chars.rewind(1);
                    Ok(consume_numeric_token(chars))
                } else {
                    Ok(CssToken::Delim(c))
                }
            }
            ':' => Ok(CssToken::Colon),
            ';' => Ok(CssToken::Semicolon),
            '<' => {
                if chars.peek_chunk(3) == [Some(&'!'), Some(&'-'), Some(&'-')] {
                    chars.next();
                    chars.next();
                    chars.next();
                    Ok(CssToken::Cdo)
                } else {
                    Ok(CssToken::Delim(c))
                }
            }
            '@' => {
                if starts_ident(&chars.peek_chunk(3)) {
                    Ok(CssToken::AtKeyword(consume_ident_sequence(chars)))
                } else {
                    Ok(CssToken::Delim(c))
                }
            }
            '[' => Ok(CssToken::OpenSquareBracket),
            '\\' => {
                if is_valid_escape(&[chars.get_last_consumed(), chars.peek()]) {
                    chars.rewind(1);
                    Ok(consume_ident_like_sequence(chars))
                } else {
                    eprintln!("parse error: invalid escape in consume_token");
                    Ok(CssToken::Delim(c))
                }
            }
            ']' => Ok(CssToken::CloseSquareBracket),
            '{' => Ok(CssToken::OpenCurlyBrace),
            '}' => Ok(CssToken::CloseCurlyBrace),
            '0'..='9' => {
                chars.rewind(1);
                Ok(consume_numeric_token(chars))
            }
            c if is_ident_start_char(c) => {
                chars.rewind(1);
                Ok(consume_ident_like_sequence(chars))
            }
            _ => Ok(CssToken::Delim(c)),
        },
        None => Ok(CssToken::Eof),
    }
}

/// https://www.w3.org/TR/css-syntax-3/#consume-string-token
fn consume_string_token(chars: &mut TokenIterator<char>, ending_char: char) -> CssToken {
    let mut string = String::new();

    loop {
        match chars.next() {
            Some(c) => match c {
                c if c == ending_char => {
                    return CssToken::String(string);
                }
                '\n' => {
                    eprintln!("parse error: newline in consume_string_token");
                    chars.rewind(1);
                    return CssToken::BadString;
                }
                '\\' => {
                    if chars.peek().is_some() {
                        if *chars.peek().unwrap() == '\n' {
                            chars.next();
                        } else {
                            string.push(consume_escaped_char(chars));
                        }
                    }
                }
                _ => {
                    string.push(*chars.get_last_consumed().unwrap());
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
fn consume_numeric_token(chars: &mut TokenIterator<char>) -> CssToken {
    let number = consume_number(chars);

    if starts_ident(&chars.peek_chunk(3)) {
        CssToken::Dimension(number, consume_ident_sequence(chars))
    } else if chars.peek() == Some(&'%') {
        chars.next();
        CssToken::Percentage(match number {
            NumericType::Integer(i) => i as f32,
            NumericType::Number(f) => f,
        })
    } else {
        CssToken::Number(number)
    }
}

/// https://www.w3.org/TR/css-syntax-3/#consume-comment
fn consume_comments(chars: &mut TokenIterator<char>) -> Result<()> {
    let mut end_with_eof = false;

    loop {
        if chars.peek_chunk(2) != [Some(&'/'), Some(&'*')] {
            break;
        }
        end_with_eof = false;
        let mut consumed_asterisk = false;
        loop {
            let c = chars.next();
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
fn consume_ident_like_sequence(chars: &mut TokenIterator<char>) -> CssToken {
    let string = consume_ident_sequence(chars);
    if string.eq_ignore_ascii_case("url") && chars.peek() == Some(&'(') {
        chars.next();
        while chars
            .peek_chunk(2)
            .iter()
            .all(|c| c.is_some_and(|c| is_whitespace(*c)))
        {
            chars.next();
        }
        match chars.peek_chunk(2)[..] {
            [Some(&'"' | &'\''), _] | [Some(&' '), Some(&'"' | &'\'')] => {
                CssToken::Function(string)
            }
            _ => consume_url_token(chars),
        }
    } else if chars.peek() == Some(&'(') {
        chars.next();
        CssToken::Function(string)
    } else {
        CssToken::Ident(string)
    }
}

/// https://www.w3.org/TR/css-syntax-3/#consume-url-token
fn consume_url_token(chars: &mut TokenIterator<char>) -> CssToken {
    let mut url = String::new();
    while chars.peek().is_some_and(|c| is_whitespace(*c)) {
        chars.next();
    }
    loop {
        match chars.next() {
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
                consume_remnants_of_bad_url(chars);
                return CssToken::BadUrl;
            }
            Some('\\') => {
                if is_valid_escape(&[chars.get_last_consumed(), chars.peek()]) {
                    url.push(consume_escaped_char(chars));
                } else {
                    eprintln!("parse error: invalid escape in consume_url_token");
                    consume_remnants_of_bad_url(chars);
                    return CssToken::BadUrl;
                }
            }
            c if c.is_some_and(is_whitespace) => {
                while chars.peek().is_some_and(|c| is_whitespace(*c)) {
                    chars.next();
                }
                if chars.peek().is_none() {
                    eprintln!("parse error: EOF in consume_url_token");
                }
                if let Some(')') | None = chars.peek() {
                    chars.next();
                    return CssToken::Url(url);
                }
                consume_remnants_of_bad_url(chars);
                return CssToken::BadUrl;
            }
            None => {
                eprintln!("parse error: EOF in consume_url_token");
                return CssToken::Url(url);
            }
            _ => {
                url.push(*chars.get_last_consumed().unwrap());
            }
        }
    }
}

/// https://www.w3.org/TR/css-syntax-3/#consume-name
fn consume_ident_sequence(chars: &mut TokenIterator<char>) -> String {
    let mut result = String::new();
    loop {
        let c = chars.next();
        match c {
            Some(c) if is_ident_char(c) => {
                result.push(c);
            }
            _ => {
                if is_valid_escape(&[chars.get_last_consumed(), chars.peek()]) {
                    result.push(consume_escaped_char(chars));
                } else {
                    chars.rewind(1);
                    return result;
                }
            }
        }
    }
}

/// https://www.w3.org/TR/css-syntax-3/#consume-escaped-code-point
fn consume_escaped_char(chars: &mut TokenIterator<char>) -> char {
    match chars.next() {
        Some(c) if c.is_ascii_hexdigit() => {
            unimplemented!()
        }
        None => {
            eprintln!("parse error: EOF in consume_escaped_char");
            '\u{FFFD}'
        }
        _ => *chars.get_last_consumed().unwrap(),
    }
}

/// https://www.w3.org/TR/css-syntax-3/#consume-number
fn consume_number(chars: &mut TokenIterator<char>) -> NumericType {
    let mut repr = String::new();
    let mut type_flag = TypeFlag::Integer;

    if let Some('+' | '-') = chars.peek() {
        repr.push(chars.next().unwrap());
    }

    while let Some('0'..='9') = chars.peek() {
        repr.push(chars.next().unwrap());
    }

    if let [Some('.'), Some('0'..='9')] = chars.peek_chunk(2)[..] {
        repr.push(chars.next().unwrap());
        repr.push(chars.next().unwrap());
        while let Some('0'..='9') = chars.peek() {
            repr.push(chars.next().unwrap());
        }
        type_flag = TypeFlag::Number;
    }

    if let [Some('E' | 'e'), Some('0'..='9')] = chars.peek_chunk(2)[..] {
        repr.push(chars.next().unwrap());
        repr.push(chars.next().unwrap());
        while let Some('0'..='9') = chars.peek() {
            repr.push(chars.next().unwrap());
        }
        type_flag = TypeFlag::Number;
    } else if let [Some('E' | 'e'), Some('+' | '-'), Some('0'..='9')] = chars.peek_chunk(3)[..] {
        repr.push(chars.next().unwrap());
        repr.push(chars.next().unwrap());
        repr.push(chars.next().unwrap());
        while let Some('0'..='9') = chars.peek() {
            repr.push(chars.next().unwrap());
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
fn consume_remnants_of_bad_url(chars: &mut TokenIterator<char>) {
    loop {
        match chars.next() {
            Some(')') | None => return,
            _ if is_valid_escape(&[chars.get_last_consumed(), chars.peek()]) => {
                consume_escaped_char(chars);
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
    is_ident_start_char(c) || c.is_ascii_digit() || c == '-'
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
        (Some('-'), Some(c), _) if is_ident_start_char(*c) => true,
        (Some('-'), c1, c2) if is_valid_escape(&[c1, c2]) => true,
        (Some('\\'), c, _) if is_valid_escape(&[Some(&'\\'), c]) => true,
        (Some(c), _, _) if is_ident_start_char(*c) => true,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consume_valid_comment() {
        let css = "/* hello, world! */";
        assert_eq!(tokenize_css(css).unwrap(), vec![CssToken::Eof]);

        let css = "/* hello, world! *//* Hello, World! */";
        assert_eq!(tokenize_css(css).unwrap(), vec![CssToken::Eof]);
    }

    #[test]
    #[should_panic]
    fn consume_invalid_comment() {
        let css = "/* hello, world!";
        assert_eq!(tokenize_css(css).unwrap(), vec![CssToken::Eof]);
    }

    #[test]
    fn tokenize_number_with_whitespace() {
        let css = "12345 67890";
        assert_eq!(
            tokenize_css(css).unwrap(),
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
            tokenize_css(css).unwrap(),
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
            tokenize_css(css).unwrap(),
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
            tokenize_css(css).unwrap(),
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
            tokenize_css(css).unwrap(),
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
