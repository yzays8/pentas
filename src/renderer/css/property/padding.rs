use std::fmt;

use anyhow::{bail, Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::property::font_size::{self, FontSizePx};
use crate::renderer::css::tokenizer::CssToken;
use crate::renderer::css::tokenizer::NumericType;

#[derive(Clone, Debug)]
pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Default for Padding {
    fn default() -> Padding {
        Padding {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

impl fmt::Display for Padding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.top, self.right, self.bottom, self.left
        )
    }
}

impl Padding {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn parse(value: &[ComponentValue], parent_px: Option<FontSizePx>) -> Result<Self> {
        if value.len() != 1 {
            unimplemented!();
        }
        let parent_px = match parent_px {
            Some(px) => px.size,
            None => font_size::MEDIUM,
        };
        let value = &value[0];
        match value {
            ComponentValue::PreservedToken(token) => match &token {
                CssToken::Ident(size) => match size.as_str() {
                    "auto" => {
                        unimplemented!();
                    }
                    _ => {
                        bail!("Invalid margin declaration: {:?}", value)
                    }
                },
                CssToken::Dimension(size, unit) => {
                    let size = match size {
                        NumericType::Integer(integer) => *integer as f32,
                        NumericType::Number(float) => *float,
                    };
                    match unit.as_str() {
                        "px" => Ok(Self::new(size, size, size, size)),
                        "em" => {
                            let size = size * parent_px;
                            Ok(Self::new(size, size, size, size))
                        }
                        _ => unimplemented!(),
                    }
                }
                CssToken::Percentage(size) => {
                    let size = *size * parent_px / 100.0;
                    Ok(Self::new(size, size, size, size))
                }
                _ => {
                    unimplemented!();
                }
            },
            _ => {
                unimplemented!();
            }
        }
    }
}
