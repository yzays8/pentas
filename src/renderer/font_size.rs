use anyhow::{bail, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::tokenizer::CssToken;
use crate::renderer::css::tokenizer::NumericType;

pub const SMALL: f32 = 13.0;
pub const MEDIUM: f32 = 16.0;
pub const LARGE: f32 = 18.0;

/// Computed value of `font-size` property. The unit is `px`.
#[derive(Clone, Copy, Debug)]
pub struct FontSizePx {
    pub size: f32,
}

impl FontSizePx {
    pub fn new(size: f32) -> Self {
        Self { size }
    }

    /// Computes the `font-size` value.
    /// The parent `font-size` value is used to calculate the relative font-size.
    pub fn parse(value: &[ComponentValue], parent_px: Option<FontSizePx>) -> Result<Self> {
        let parent_px = match parent_px {
            Some(px) => px.size,
            None => MEDIUM,
        };
        if value.len() != 1 {
            bail!("Invalid font-size declaration: {:?}", value);
        }
        let value = &value[0];
        match value {
            ComponentValue::PreservedToken(token) => match &token {
                CssToken::Ident(size) => match size.as_str() {
                    "small" | "medium" | "large" => Ok(Self::new(match size.as_str() {
                        "small" => SMALL,
                        "medium" => MEDIUM,
                        "large" => LARGE,
                        _ => unreachable!(),
                    })),
                    _ => {
                        unimplemented!();
                    }
                },
                CssToken::Dimension(size, unit) => {
                    let size = match size {
                        NumericType::Integer(integer) => *integer as f32,
                        NumericType::Number(float) => *float,
                    };
                    match unit.as_str() {
                        "px" => Ok(Self::new(size)),
                        "em" => Ok(Self::new(size * parent_px)),
                        _ => unimplemented!(),
                    }
                }
                CssToken::Percentage(size) => Ok(Self::new(size / 100.0 * parent_px)),
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_size() {
        let parent_px = FontSizePx::new(16.0);

        let value = vec![ComponentValue::PreservedToken(CssToken::Dimension(
            NumericType::Number(12.0),
            "px".to_string(),
        ))];
        assert_eq!(
            FontSizePx::parse(&value, Some(parent_px)).unwrap().size,
            12.0
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Dimension(
            NumericType::Number(1.5),
            "em".to_string(),
        ))];
        assert_eq!(
            FontSizePx::parse(&value, Some(parent_px)).unwrap().size,
            24.0
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Percentage(50.0))];
        assert_eq!(
            FontSizePx::parse(&value, Some(parent_px)).unwrap().size,
            8.0
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "small".to_string(),
        ))];
        assert_eq!(
            FontSizePx::parse(&value, Some(parent_px)).unwrap().size,
            SMALL
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "medium".to_string(),
        ))];
        assert_eq!(
            FontSizePx::parse(&value, Some(parent_px)).unwrap().size,
            MEDIUM
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "large".to_string(),
        ))];
        assert_eq!(
            FontSizePx::parse(&value, Some(parent_px)).unwrap().size,
            LARGE
        );
    }
}
