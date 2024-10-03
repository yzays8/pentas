pub mod cssom;
pub mod dtype;
pub mod parser;
pub mod property;
pub mod selector;
pub mod tokenizer;

use anyhow::Result;

use cssom::StyleSheet;
use parser::CssParser;
use tokenizer::CssTokenizer;

const UA_CSS_PATH: &str = "src/renderer/css/ua.css";

/// Returns the user agent style sheet.
pub fn get_ua_style_sheet() -> Result<StyleSheet> {
    let ua_css = std::fs::read_to_string(UA_CSS_PATH)?;
    CssParser::new(CssTokenizer::new(&ua_css).tokenize()?).parse()
}
