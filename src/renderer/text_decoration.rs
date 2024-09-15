use std::fmt;

use anyhow::{Ok, Result};

use crate::renderer::color::Color;
use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::tokenizer::CssToken;

#[derive(Clone, Debug)]
pub struct TextDecoration {
    pub color: Color,
    pub line: TextDecorationLine,
    pub style: TextDecorationStyle,
}

#[derive(Clone, Debug)]
pub enum TextDecorationLine {
    None,
    Underline,
    // Overline,
    // LineThrough,
}

#[derive(Clone, Debug)]
pub enum TextDecorationStyle {
    Solid,
    // Double,
    // Dotted,
    // Dashed,
    // Wavy,
}

impl Default for TextDecoration {
    fn default() -> TextDecoration {
        TextDecoration {
            color: Color::default(),
            line: TextDecorationLine::None,
            style: TextDecorationStyle::Solid,
        }
    }
}

impl fmt::Display for TextDecoration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {:?} {:?}", self.color, self.line, self.style)
    }
}

impl TextDecoration {
    pub fn new(color: Color, line: TextDecorationLine, style: TextDecorationStyle) -> Self {
        Self { color, line, style }
    }

    pub fn parse(values: &[ComponentValue], current_color: &Color) -> Result<Self> {
        if values.len() != 1 {
            unimplemented!();
        }
        if values.first().unwrap()
            == &ComponentValue::PreservedToken(CssToken::Ident("underline".to_string()))
        {
            Ok(Self::new(
                *current_color,
                TextDecorationLine::Underline,
                TextDecorationStyle::Solid,
            ))
        } else {
            unimplemented!();
        }
    }
}
