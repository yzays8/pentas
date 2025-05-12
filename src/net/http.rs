use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
};

use anyhow::{Context, Result, anyhow};
use native_tls::TlsConnector;

/// HTTP/1.1 Request
#[allow(dead_code)]
#[derive(Debug)]
pub struct HttpRequest {
    method: String,
    path: String,
    host: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

impl HttpRequest {
    #[allow(dead_code)]
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.headers.push((key.to_string(), value.to_string()));
    }

    // HTTP-message   = start-line CRLF
    //                  *( field-line CRLF )
    //                  CRLF
    //                  [ message-body ]
    /// https://datatracker.ietf.org/doc/html/rfc9112#section-2.1
    pub fn to_http_format(&self) -> String {
        let mut request = format!("{} {} HTTP/1.1\r\n", self.method, self.path);
        for (key, value) in &self.headers {
            request.push_str(&format!("{}: {}\r\n", key, value));
        }
        request.push_str("\r\n");
        if let Some(body) = &self.body {
            request.push_str(body);
        }
        request
    }
}

/// HTTP/1.1 Response
#[allow(dead_code)]
#[derive(Debug)]
pub struct HttpResponse {
    pub status_line: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl HttpResponse {
    // HTTP-message   = start-line CRLF
    //                  *( field-line CRLF )
    //                  CRLF
    //                  [ message-body ]
    /// https://datatracker.ietf.org/doc/html/rfc9112#section-2.1
    pub fn from_str(response_text: &str) -> Result<Self> {
        let mut lines = response_text.split("\r\n");
        let status_line = lines.next().context(anyhow!("No status line"))?.to_string();

        let mut headers = Vec::new();
        for line in lines.by_ref() {
            if line.is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(": ") {
                headers.push((key.to_string(), value.to_string()));
            }
        }

        let body = lines.collect::<Vec<&str>>().join("\r\n");

        Ok(Self {
            status_line,
            headers,
            body,
        })
    }
}

/// HTTP/1.1 Client
#[derive(Debug)]
pub struct HttpClient {
    host: String,
    port: u16,
}

impl HttpClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
        }
    }

    pub fn send_request(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<&str>,
        use_https: bool,
    ) -> Result<HttpResponse> {
        let addr = format!("{}:{}", self.host, self.port)
            // This is where the DNS resolution takes place.
            // note: This is a blocking operation.
            .to_socket_addrs()?
            .next()
            .context(anyhow!("Failed to resolve address"))?;

        let request = HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            host: self.host.clone(),
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: body.map(|s| s.to_string()),
        };
        let mut stream = TcpStream::connect((addr.ip(), addr.port()))?;
        let response_text = if use_https {
            let mut tls_stream = TlsConnector::new()?.connect(&self.host, stream)?;
            tls_stream.write_all(request.to_http_format().as_bytes())?;
            tls_stream.flush()?;
            let mut response_text = String::new();
            tls_stream.read_to_string(&mut response_text)?;
            response_text
        } else {
            stream.write_all(request.to_http_format().as_bytes())?;
            stream.flush()?;
            let mut response_text = String::new();
            stream.read_to_string(&mut response_text)?;
            response_text
        };

        HttpResponse::from_str(&response_text)
    }
}
