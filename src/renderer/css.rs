pub mod cssom;
mod parser;
pub mod selector;
pub mod token;

pub use self::{parser::CssParser, token::CssTokenizer};

use anyhow::Result;

use self::cssom::StyleSheet;

const UA_CSS_PATH: &str = "assets/style/ua.css";

/// Returns the user agent style sheet.
pub fn get_ua_style_sheet() -> Result<StyleSheet> {
    let css = std::fs::read_to_string(UA_CSS_PATH)?;
    CssParser::new(&CssTokenizer::new(&css).tokenize()?).parse()
}
