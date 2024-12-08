pub mod cssom;
pub mod parser;
pub mod selector;
pub mod token;

use anyhow::Result;

use cssom::StyleSheet;
use parser::parse_css;
use token::CssTokenizer;

const UA_CSS_PATH: &str = "src/renderer/style/ua.css";

/// Returns the user agent style sheet.
pub fn get_ua_style_sheet() -> Result<StyleSheet> {
    let css = std::fs::read_to_string(UA_CSS_PATH)?;
    parse_css(&CssTokenizer::new(&css).tokenize()?)
}
