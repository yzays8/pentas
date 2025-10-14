use anyhow::{Context, Ok, Result, bail};

use crate::utils::TokenIterator;

/// https://url.spec.whatwg.org/#url-representation
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct Url {
    pub scheme: String,
    pub username: String,
    pub password: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    /// A URL path is either a URL path segment or a list of zero or more URL path segments,
    /// but for simplicity we will always treat it as a list.
    /// https://url.spec.whatwg.org/#url-path
    pub path: Vec<String>,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

impl Url {
    /// https://url.spec.whatwg.org/#url-parsing
    pub fn from_str(url: &str) -> Result<Self> {
        UrlParser::new(url).parse()
    }

    /// https://url.spec.whatwg.org/#special-scheme
    fn has_special_scheme(&self) -> bool {
        matches!(
            self.scheme.as_str(),
            "ftp" | "file" | "http" | "https" | "ws" | "wss"
        )
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum UrlParsingState {
    #[default]
    SchemeStart,
    Scheme,
    NoScheme,
    File,
    FileSlash,
    FileHost,
    PathOrAuthority,
    SpecialAuthoritySlashes,
    SpecialAuthorityIgnoreSlashes,
    Authority,
    Host,
    Port,
    PathStart,
    Path,
    OpaquePath,
    Query,
    Fragment,
}

/// This is a very limited version of the URL parser from the WHATWG URL standard.
/// It does not take a base URL as an argument, so only absolute URLs are supported for now.
/// https://url.spec.whatwg.org/#concept-url-parser
#[derive(Debug)]
struct UrlParser {
    input: TokenIterator<char>,
    output: Url,
    state: UrlParsingState,
    buf: String,
}

impl UrlParser {
    pub fn new(url: &str) -> Self {
        Self {
            input: TokenIterator::new(
                &url.replace("\t", "")
                    .replace("\n", "")
                    .replace("\r", "")
                    .chars()
                    .collect::<Vec<char>>(),
            ),
            output: Url::default(),
            state: UrlParsingState::SchemeStart,
            buf: String::new(),
        }
    }

    /// https://url.spec.whatwg.org/#url-parsing
    pub fn parse(&mut self) -> Result<Url> {
        loop {
            match self.state {
                UrlParsingState::SchemeStart => self.parse_scheme_start()?,
                UrlParsingState::Scheme => self.parse_scheme()?,
                UrlParsingState::NoScheme => self.parse_no_scheme()?,
                UrlParsingState::File => self.parse_file()?,
                UrlParsingState::FileSlash => self.parse_file_slash()?,
                UrlParsingState::FileHost => self.parse_file_host()?,
                UrlParsingState::PathOrAuthority => self.parse_path_or_authority()?,
                UrlParsingState::SpecialAuthoritySlashes => {
                    self.parse_special_authority_slashes()?
                }
                UrlParsingState::SpecialAuthorityIgnoreSlashes => {
                    self.parse_special_authority_ignore_slashes()?
                }
                UrlParsingState::Authority => self.parse_authority()?,
                UrlParsingState::Host => self.parse_host()?,
                UrlParsingState::Port => self.parse_port()?,
                UrlParsingState::PathStart => self.parse_path_start()?,
                UrlParsingState::Path => self.parse_path()?,
                UrlParsingState::OpaquePath => self.parse_opaque_path()?,
                UrlParsingState::Query => self.parse_query()?,
                UrlParsingState::Fragment => self.parse_fragment()?,
            }

            if self.input.next().is_none() {
                break;
            }
        }

        Ok(self.output.clone())
    }

    /// https://url.spec.whatwg.org/#scheme-start-state
    fn parse_scheme_start(&mut self) -> Result<()> {
        match self.input.peek() {
            Some(c) if c.is_ascii_alphabetic() => {
                self.buf.push(c.to_ascii_lowercase());
                self.state = UrlParsingState::Scheme;
            }
            _ => {
                self.state = UrlParsingState::NoScheme;
                self.input.rewind(1);
            }
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#scheme-state
    fn parse_scheme(&mut self) -> Result<()> {
        let c = self.input.peek();
        match c {
            Some(c) if c.is_ascii_alphanumeric() => {
                self.buf.push(c.to_ascii_lowercase());
            }
            Some('+' | '-' | '.') => {
                self.buf.push(c.unwrap().to_ascii_lowercase());
            }
            Some(':') => {
                self.output.scheme = self.buf.clone();
                self.buf.clear();
                if self.output.scheme == "file" {
                    if let [_, Some('/'), Some('/')] = self.input.peek_chunk(3).as_slice() {
                        self.state = UrlParsingState::File;
                    } else {
                        bail!("special-scheme-missing-following-solidus validation error");
                    }
                } else if self.output.has_special_scheme() {
                    self.state = UrlParsingState::SpecialAuthoritySlashes;
                } else if self.input.peek() == Some(&'/') {
                    self.state = UrlParsingState::PathOrAuthority;
                    self.input.forward(1);
                } else {
                    self.output.path = vec!["".to_string()];
                    self.state = UrlParsingState::OpaquePath;
                }
            }
            _ => {
                self.buf.clear();
                self.state = UrlParsingState::NoScheme;
                unimplemented!();
            }
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#no-scheme-state
    fn parse_no_scheme(&mut self) -> Result<()> {
        bail!("missing-scheme-non-relative-URL validation error")
    }

    /// https://url.spec.whatwg.org/#file-state
    fn parse_file(&mut self) -> Result<()> {
        self.output.scheme = "file".to_string();
        self.output.host = Some("".to_string());
        let c = self.input.peek();
        if let Some('/' | '\\') = c {
            if let Some('\\') = c {
                eprintln!("invalid-reverse-solidus validation error");
            }
            self.state = UrlParsingState::FileSlash;
        } else {
            self.state = UrlParsingState::Path;
            self.input.rewind(1);
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#file-slash-state
    fn parse_file_slash(&mut self) -> Result<()> {
        let c = self.input.peek();
        if let Some('/' | '\\') = c {
            if let Some('\\') = c {
                eprintln!("invalid-reverse-solidus validation error");
            }
            self.state = UrlParsingState::FileHost;
        } else {
            self.state = UrlParsingState::Path;
            self.input.rewind(1);
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#file-host-state
    fn parse_file_host(&mut self) -> Result<()> {
        let c = self.input.peek();
        if matches!(c, None | Some('/' | '\\' | '?' | '#')) {
            self.input.rewind(1);
            if self.buf.is_empty() {
                self.output.host = Some("".to_string());
                self.state = UrlParsingState::PathStart;
            } else {
                if self.buf == "localhost" {
                    self.output.host = Some("".to_string());
                } else {
                    self.output.host = Some(self.buf.clone());
                }
                self.buf.clear();
                self.state = UrlParsingState::PathStart;
            }
        } else {
            self.buf.push(*c.unwrap());
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#path-or-authority-state
    fn parse_path_or_authority(&mut self) -> Result<()> {
        if let Some('/') = self.input.peek() {
            unimplemented!()
        } else {
            self.state = UrlParsingState::Path;
            self.input.rewind(1);
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#special-authority-slashes-state
    fn parse_special_authority_slashes(&mut self) -> Result<()> {
        if let [Some('/'), Some('/')] = self.input.peek_chunk(2).as_slice() {
            self.state = UrlParsingState::SpecialAuthorityIgnoreSlashes;
            self.input.forward(1);
        } else {
            eprintln!("special-scheme-missing-following-solidus validation error");
            self.state = UrlParsingState::SpecialAuthorityIgnoreSlashes;
            self.input.rewind(1);
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#special-authority-ignore-slashes-state
    fn parse_special_authority_ignore_slashes(&mut self) -> Result<()> {
        let c = self.input.peek();
        if !matches!(c, Some('/' | '\\')) {
            self.state = UrlParsingState::Authority;
            self.input.rewind(1);
        } else {
            bail!("special-scheme-missing-following-solidus validation error");
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#authority-state
    fn parse_authority(&mut self) -> Result<()> {
        let c = self.input.peek();
        if let Some('@') = c {
            unimplemented!();
        } else if matches!(c, None | Some('/' | '?' | '#'))
            || (self.output.has_special_scheme() && c == Some(&'\\'))
        {
            self.input.rewind(self.buf.len() + 1);
            self.buf.clear();
            self.state = UrlParsingState::Host;
        } else {
            self.buf.push(*c.unwrap());
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#host-state
    fn parse_host(&mut self) -> Result<()> {
        let c = self.input.peek();
        if let Some(':') = c {
            if self.buf.is_empty() {
                bail!("host-missing validation error");
            }
            // todo: Use the host parser. https://url.spec.whatwg.org/#host-parsing
            self.output.host = Some(self.buf.clone());
            self.buf.clear();
            self.state = UrlParsingState::Port;
        } else if matches!(c, None | Some('/' | '?' | '#'))
            || (self.output.has_special_scheme() && c == Some(&'\\'))
        {
            if self.output.has_special_scheme() && self.buf.is_empty() {
                bail!("host-missing validation error");
            }
            self.output.host = Some(self.buf.clone());
            self.buf.clear();
            self.state = UrlParsingState::PathStart;
        } else {
            if let Some('[' | ']') = c {
                unimplemented!();
            }
            self.buf.push(*c.unwrap());
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#port-state
    fn parse_port(&mut self) -> Result<()> {
        let c = self.input.peek();
        if let Some(c) = c
            && c.is_ascii_digit()
        {
            self.buf.push(*c);
            return Ok(());
        }
        if matches!(c, None | Some('/' | '?' | '#'))
            || (self.output.has_special_scheme() && c == Some(&'\\'))
        {
            if !self.buf.is_empty() {
                let port = self
                    .buf
                    .parse::<u16>()
                    .context("port-out-of-range validation error")?;
                let is_default_port = match self.output.scheme.as_str() {
                    "ftp" => port == 21,
                    "http" | "ws" => port == 80,
                    "https" | "wss" => port == 443,
                    _ => false,
                };
                if is_default_port {
                    self.output.port = None;
                } else {
                    self.output.port = Some(port);
                }
                self.buf.clear();
            }
            self.state = UrlParsingState::PathStart;
            self.input.rewind(1);
        } else {
            bail!("port-invalid validation error");
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#path-start-state
    fn parse_path_start(&mut self) -> Result<()> {
        let c = self.input.peek();
        if self.output.has_special_scheme() {
            if let Some('\\') = c {
                eprintln!("invalid-reverse-solidus validation error");
            }
            self.state = UrlParsingState::Path;
            if !matches!(c, Some('/' | '\\')) {
                self.input.rewind(1);
            }
        } else if let Some('?') = c {
            self.output.query = Some("".to_string());
            self.state = UrlParsingState::Query;
        } else if let Some('#') = c {
            self.output.fragment = Some("".to_string());
            self.state = UrlParsingState::Fragment;
        } else if c.is_some() {
            self.state = UrlParsingState::Path;
            if let Some('/') = c {
                self.input.rewind(1);
            }
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#path-state
    fn parse_path(&mut self) -> Result<()> {
        let c = self.input.peek();
        if matches!(c, None | Some('/' | '?' | '#'))
            || (self.output.has_special_scheme() && c == Some(&'\\'))
        {
            if self.output.has_special_scheme() && c == Some(&'\\') {
                bail!("invalid-reverse-solidus validation error");
            }
            match self.buf.as_str() {
                ".." => unimplemented!(),
                "." => {
                    if !self.output.has_special_scheme() && c == Some(&'\\') {
                        self.output.path.push(self.buf.clone());
                    }
                }
                _ => {
                    self.output.path.push(self.buf.clone());
                }
            }
            self.buf.clear();
            if let Some('?') = c {
                self.output.query = Some("".to_string());
                self.state = UrlParsingState::Query;
            } else if let Some('#') = c {
                self.output.fragment = Some("".to_string());
                self.state = UrlParsingState::Fragment;
            }
        } else {
            self.buf.push(*c.unwrap());
        }

        Ok(())
    }

    /// https://url.spec.whatwg.org/#cannot-be-a-base-url-path-state
    fn parse_opaque_path(&mut self) -> Result<()> {
        let c = self.input.peek();
        match c {
            Some('?') => {
                self.output.query = Some("".to_string());
                self.state = UrlParsingState::Query;
            }
            Some('#') => {
                self.output.fragment = Some("".to_string());
                self.state = UrlParsingState::Fragment;
            }
            Some(' ') => {
                if let [_, Some('?'), Some('#')] = self.input.peek_chunk(3).as_slice() {
                    self.output.path.get_mut(0).unwrap().push_str("%20");
                } else {
                    self.output.path.get_mut(0).unwrap().push(' ');
                }
            }
            Some(c) => {
                self.output.path.get_mut(0).unwrap().push(*c);
            }
            _ => {}
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#query-state
    fn parse_query(&mut self) -> Result<()> {
        let c = self.input.peek();
        if let Some('#') | None = c {
            self.output.query = Some(self.buf.clone());
            self.buf.clear();
            if let Some('#') = c {
                self.output.fragment = Some("".to_string());
                self.state = UrlParsingState::Fragment;
            }
        } else {
            self.buf.push(*c.unwrap());
        }
        Ok(())
    }

    /// https://url.spec.whatwg.org/#fragment-state
    fn parse_fragment(&mut self) -> Result<()> {
        let c = self.input.peek();
        if let Some(c) = c {
            self.output.fragment =
                Some(self.output.fragment.clone().unwrap_or_default() + &c.to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_url() {
        let url = "https://example.com/";
        let url = Url::from_str(url).unwrap();
        assert_eq!(url.scheme, "https");
        assert_eq!(url.username, "");
        assert_eq!(url.password, "");
        assert_eq!(url.host, Some("example.com".to_string()));
        assert_eq!(url.port, None);
        assert_eq!(url.path, vec![""]);
        assert_eq!(url.query, None);
        assert_eq!(url.fragment, None);
    }

    #[test]
    fn full_url_with_query_and_fragment() {
        let url = "https://localhost:8000/search?q=text#hello";
        let url = Url::from_str(url).unwrap();
        assert_eq!(url.scheme, "https");
        assert_eq!(url.username, "");
        assert_eq!(url.password, "");
        assert_eq!(url.host, Some("localhost".to_string()));
        assert_eq!(url.port, Some(8000));
        assert_eq!(url.path, vec!["search"]);
        assert_eq!(url.query, Some("q=text".to_string()));
        assert_eq!(url.fragment, Some("hello".to_string()));
    }

    #[test]
    fn opaque_url() {
        let url = "urn:isbn:9780307476463";
        let url = Url::from_str(url).unwrap();
        assert_eq!(url.scheme, "urn");
        assert_eq!(url.username, "");
        assert_eq!(url.password, "");
        assert_eq!(url.host, None);
        assert_eq!(url.port, None);
        assert_eq!(url.path, vec!["isbn:9780307476463"]);
        assert_eq!(url.query, None);
        assert_eq!(url.fragment, None);
    }

    #[test]
    fn percent_encoded_file_url_path() {
        let url = "file:///ada/Analytical%20Engine/README.md";
        let url = Url::from_str(url).unwrap();
        assert_eq!(url.scheme, "file");
        assert_eq!(url.username, "");
        assert_eq!(url.password, "");
        assert_eq!(url.host, Some("".to_string()));
        assert_eq!(url.port, None);
        assert_eq!(url.path, vec!["ada", "Analytical%20Engine", "README.md"]);
        assert_eq!(url.query, None);
        assert_eq!(url.fragment, None);
    }
}
