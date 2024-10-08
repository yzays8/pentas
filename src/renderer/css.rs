pub mod cssom;
pub mod dtype;
pub mod parser;
pub mod property;
pub mod selector;
pub mod token;

use anyhow::Result;

use cssom::StyleSheet;
use parser::parse;
use token::tokenize;

const UA_CSS_PATH: &str = "src/renderer/css/ua.css";

/// Returns the user agent style sheet.
pub fn get_ua_style_sheet() -> Result<StyleSheet> {
    let ua_css = std::fs::read_to_string(UA_CSS_PATH)?;
    parse(&tokenize(&ua_css)?)
}
