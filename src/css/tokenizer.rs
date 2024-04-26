/// https://www.w3.org/TR/css-syntax-3/#tokenization
#[derive(Debug)]
pub enum Token {
    Ident(String),
    Function(String),
    AtKeyword(String),
    Hash(String),
    String(String),
    BadString(String),
    Url(String),
    BadUrl(String),
    Delim(char),
    Number(f64),
    Percentage(f64),
    Dimension(f64, f64),
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
}

#[derive(Debug)]
pub struct Tokenizer {
    input: Vec<char>,
    current_pos: usize,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            current_pos: 0,
        }
    }

    pub fn consume_token(&self) -> Vec<Token> {
        let mut tokens = vec![];
        tokens
    }

    fn consume_char(&mut self) -> Option<char> {
        self.input.get(self.current_pos).map(|c| {
            self.current_pos += 1;
            *c
        })
    }
}
