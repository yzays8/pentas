use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("{0}")]
    HtmlParse(String),
    #[error("{0}")]
    CssTokenize(String),
    #[error("{0}")]
    CssParse(String),
    #[error("{0}")]
    CssSelectorParse(String),
    #[error("{0}")]
    UrlParse(String),
    #[error("{0}")]
    Network(String),
    #[error(transparent)]
    Tls(#[from] native_tls::Error),
    #[error(transparent)]
    TlsHandshake(#[from] native_tls::HandshakeError<std::net::TcpStream>),
    #[error("{0}")]
    Style(String),
    #[error("{0}")]
    CssProperty(String),
    #[error("{0}")]
    Layout(String),
    #[error(transparent)]
    GtkBool(#[from] gtk4::glib::BoolError),
    #[error(transparent)]
    Regex(#[from] regex::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
