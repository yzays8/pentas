#![allow(dead_code)]

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
};

use native_tls::TlsConnector;

use crate::{
    error::{Error, Result},
    net::url::Url,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Other(String),
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // The request method is case-sensitive.
        // https://datatracker.ietf.org/doc/html/rfc9112#section-3.1
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Other(s) => write!(f, "{}", s),
        }
    }
}

impl From<&str> for HttpMethod {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "HEAD" => HttpMethod::Head,
            other => HttpMethod::Other(other.to_string()),
        }
    }
}

/// HTTP/1.1 request
#[derive(Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

impl HttpRequest {
    pub fn builder(method: impl Into<HttpMethod>, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder {
            method: method.into(),
            url: url.into(),
            headers: Vec::new(),
            body: None,
        }
    }

    pub fn send(&self, host: &str, port: u16, use_tls: bool) -> Result<HttpResponse> {
        let mut stream: Box<dyn ReadWrite> = if use_tls {
            let tcp = TcpStream::connect((host, port))?;
            Box::new(TlsConnector::new()?.connect(host, tcp)?)
        } else {
            Box::new(TcpStream::connect((host, port))?)
        };

        stream.write_all(&self.to_bytes(host))?;
        stream.flush()?;

        HttpResponse::parse(&mut BufReader::new(stream))
    }

    /// https://datatracker.ietf.org/doc/html/rfc9112#section-2.1
    fn to_bytes(&self, host: &str) -> Vec<u8> {
        let mut buf = Vec::new();

        // https://datatracker.ietf.org/doc/html/rfc9112#section-3
        let start_line = format!("{} {} HTTP/1.1\r\n", self.method, self.path);
        buf.extend_from_slice(start_line.as_bytes());

        let mut seen = HashMap::<String, bool>::new();
        for (k, _) in &self.headers {
            seen.insert(k.to_lowercase(), true);
        }

        // A client MUST send a Host header field in an HTTP/1.1 request.
        // https://datatracker.ietf.org/doc/html/rfc9112#section-3.2.2-5
        if !seen.contains_key("host") {
            buf.extend_from_slice(format!("Host: {}\r\n", host).as_bytes());
        }

        for (k, v) in &self.headers {
            buf.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
        }

        if let Some(body) = &self.body {
            // When a message does not have a Transfer-Encoding header field, a Content-Length header
            // field can provide the anticipated size.
            // A sender MUST NOT send a Content-Length header field in any message that contains a
            // Transfer-Encoding header field.
            // https://datatracker.ietf.org/doc/html/rfc9112#section-6.2
            if !seen.contains_key("content-length") && !seen.contains_key("transfer-encoding") {
                buf.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
            }
        }

        buf.extend_from_slice(b"\r\n");

        if let Some(body) = &self.body {
            buf.extend_from_slice(body);
        }

        buf
    }
}

#[derive(Debug)]
pub struct RequestBuilder {
    method: HttpMethod,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

impl RequestBuilder {
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    pub fn body(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.body = Some(bytes.into());
        self
    }

    pub fn send(self) -> Result<HttpResponse> {
        let url = Url::from_str(&self.url)?;

        // The specific connection protocols to be used for an HTTP interaction are
        // determined by client configuration and the target URI.
        // https://datatracker.ietf.org/doc/html/rfc9112#section-9-2
        let port = url
            .port
            .unwrap_or_else(|| if url.scheme == "https" { 443 } else { 80 });

        // If the target URI's path component is empty, the client MUST send "/" as the
        // path within the origin-form of request-target.
        // https://datatracker.ietf.org/doc/html/rfc9112#section-3.2.1-2
        let path = format!("/{}", url.path.join("/"));

        let req = HttpRequest {
            method: self.method,
            path,
            headers: self.headers,
            body: self.body,
        };
        req.send(&url.host.unwrap(), port, url.scheme == "https")
    }
}

/// HTTP/1.1 response
#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        let lname = name.to_lowercase();
        for (k, v) in &self.headers {
            if k.to_lowercase() == lname {
                return Some(v.as_str());
            }
        }
        None
    }

    /// https://datatracker.ietf.org/doc/html/rfc9112#section-2.2
    pub fn parse(reader: &mut impl BufRead) -> Result<Self> {
        // https://datatracker.ietf.org/doc/html/rfc9112#name-status-line
        let mut status_line = String::new();
        reader.read_line(&mut status_line)?;
        if status_line.is_empty() {
            return Err(Error::Network("empty status line".into()));
        }

        let status_code = status_line
            .trim_end_matches(&['\r', '\n'][..])
            .to_string()
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or(Error::Network("invalid status line".into()))?;

        let mut headers = HashMap::<String, String>::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            let line = line.trim_end_matches(&['\r', '\n'][..]);
            if line.is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                // Header field names are case-insensitive.
                // https://datatracker.ietf.org/doc/html/rfc9112#section-5
                headers.insert(k.trim().to_string().to_lowercase(), v.trim().to_string());
            }
        }

        // If a message body has been indicated, then it is read as a stream until an amount of
        // octets equal to the message body length is read or the connection is closed.
        // https://datatracker.ietf.org/doc/html/rfc9112#section-2.2-1
        let mut body: Vec<u8> = Vec::new();

        // Responses with certain status codes do not include a message body.
        // https://datatracker.ietf.org/doc/html/rfc9112#section-6.3-2.1
        if matches!(status_code, 100..200 | 204 | 304) {
            return Ok(Self {
                status_code,
                headers,
                body,
            });
        }

        // If a message is received with both a Transfer-Encoding and a Content-Length header field,
        // the Transfer-Encoding overrides the Content-Length.
        // https://datatracker.ietf.org/doc/html/rfc9112#section-6.3-2.3
        if let Some(te) = headers.get("transfer-encoding") {
            match te.to_lowercase().as_str() {
                "chunked" => {
                    let body = Self::read_chunked_body(reader)?;
                    return Ok(Self {
                        status_code,
                        headers,
                        body,
                    });
                }
                _ => unimplemented!(),
            }
        }

        if let Some(cl) = headers.get("content-length") {
            // The user agent MUST close the connection to the server and discard the received response
            // if a message is received without Transfer-Encoding and with an invalid Content-Length header field.
            // https://datatracker.ietf.org/doc/html/rfc9112#section-6.3-2.5
            let len: usize = cl.parse()?;
            reader.take(len as u64).read_to_end(&mut body)?;
            return Ok(Self {
                status_code,
                headers,
                body,
            });
        }

        // Read until connection close.
        reader.read_to_end(&mut body)?;

        Ok(Self {
            status_code,
            headers,
            body,
        })
    }

    /// https://datatracker.ietf.org/doc/html/rfc9112#section-7.1
    fn read_chunked_body(reader: &mut impl BufRead) -> Result<Vec<u8>> {
        // https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.3
        let mut out = Vec::new();
        loop {
            let mut size_line = String::new();
            reader.read_line(&mut size_line)?;

            let size = usize::from_str_radix(
                size_line
                    .trim_end_matches(&['\r', '\n'][..])
                    .trim()
                    .split(';')
                    .next()
                    .ok_or(Error::Network("invalid chunk size line".into()))?,
                16,
            )?;

            // last-chunk and trailer-section
            if size == 0 {
                let mut line = String::new();
                // Ignore the trailer section for now.
                // https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.2
                loop {
                    line.clear();
                    reader.read_line(&mut line)?;
                    let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
                    if trimmed.is_empty() {
                        break;
                    }
                }
                break;
            }

            let mut chunk = vec![0u8; size];
            reader.read_exact(&mut chunk)?;
            out.extend_from_slice(&chunk);

            let mut crlf = [0u8; 2];
            reader.read_exact(&mut crlf)?;
        }

        Ok(out)
    }
}

/// HTTP/1.1 client
#[derive(Debug)]
pub struct HttpClient;

impl HttpClient {
    pub fn new() -> Self {
        Self
    }

    pub fn get(&self, url: &str) -> RequestBuilder {
        HttpRequest::builder(HttpMethod::Get, url)
    }

    pub fn post(&self, url: &str) -> RequestBuilder {
        HttpRequest::builder(HttpMethod::Post, url)
    }
}

pub fn get(url: &str) -> Result<HttpResponse> {
    HttpClient::new().get(url).send()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_transfer_encoding_chunk() {
        let chunked = b"23\r\n\
{\"id\": 0, \"message\": \"Hello world\"}\r\n\
23\r\n\
{\"id\": 1, \"message\": \"Hello again\"}\r\n\
0\r\n\
\r\n";

        let mut reader = BufReader::new(&chunked[..]);
        let actual = HttpResponse::read_chunked_body(&mut reader).unwrap();
        let expected = br#"{"id": 0, "message": "Hello world"}{"id": 1, "message": "Hello again"}"#;
        assert_eq!(&actual, expected);
    }
}
