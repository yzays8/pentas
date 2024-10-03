use std::fmt;
use std::iter::Peekable;

use anyhow::{bail, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::dtype::{
    self, parse_length_percentage_type, AbsoluteLengthUnit, CssValue, LengthUnit,
    RelativeLengthUnit,
};
use crate::renderer::css::tokenizer::CssToken;

pub const SMALL: f32 = 13.0;
pub const MEDIUM: f32 = 16.0;
pub const LARGE: f32 = 18.0;

#[derive(Clone, Debug, PartialEq)]
pub struct FontSizeProp {
    pub size: CssValue,
}

impl fmt::Display for FontSizeProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.size)
    }
}

impl FontSizeProp {
    pub fn compute(&mut self, parent_px: Option<&Self>) -> Result<&Self> {
        let parent_px = match parent_px {
            Some(FontSizeProp {
                size: CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            }) => *size,
            _ => MEDIUM,
        };

        match &self.size {
            CssValue::AbsoluteSize(size) => match size {
                dtype::AbsoluteSize::Small => {
                    self.size = CssValue::Length(
                        SMALL,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    );
                }
                dtype::AbsoluteSize::Medium => {
                    self.size = CssValue::Length(
                        MEDIUM,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    );
                }
                dtype::AbsoluteSize::Large => {
                    self.size = CssValue::Length(
                        LARGE,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    );
                }
                _ => unimplemented!(),
            },
            CssValue::RelativeSize(_) => unimplemented!(),
            CssValue::Length(size, unit) => match unit {
                LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px) => {
                    self.size = CssValue::Length(
                        *size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    );
                }
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Em) => {
                    self.size = CssValue::Length(
                        size * parent_px,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    );
                }
                _ => unimplemented!(),
            },
            CssValue::Percentage(size) => {
                self.size = CssValue::Length(
                    size / 100.0 * parent_px,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                );
            }
            _ => bail!("Invalid font-size value: {:?}", self.size),
        }

        Ok(self)
    }
}

// font-size =
//   <absolute-size>            |
//   <relative-size>            |
//   <length-percentage [0,∞]>  |
//   math
pub fn parse_font_size(values: &[ComponentValue]) -> Result<FontSizeProp> {
    let mut values = values.iter().cloned().peekable();
    if let Some(ComponentValue::PreservedToken(CssToken::Ident(size))) = values.peek() {
        match size.as_str() {
            "xx-small" | "x-small" | "small" | "medium" | "large" | "x-large" | "xx-large" => {
                Ok(FontSizeProp {
                    size: parse_absolute_size_type(&mut values)?,
                })
            }
            "larger" | "smaller" => Ok(FontSizeProp {
                size: parse_relative_size_type(&mut values)?,
            }),
            _ => bail!(
                "Expected absolute or relative size value but found: {:?}",
                values
            ),
        }
    } else {
        Ok(FontSizeProp {
            size: parse_length_percentage_type(&mut values)?,
        })
    }
}

// <absolute-size> = xx-small | x-small | small | medium | large | x-large | xx-large | xxx-large
pub fn parse_absolute_size_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    match values.next() {
        Some(v) => match &v {
            ComponentValue::PreservedToken(CssToken::Ident(size)) => match size.as_str() {
                "xx-small" => Ok(CssValue::AbsoluteSize(dtype::AbsoluteSize::XXSmall)),
                "x-small" => Ok(CssValue::AbsoluteSize(dtype::AbsoluteSize::XSmall)),
                "small" => Ok(CssValue::AbsoluteSize(dtype::AbsoluteSize::Small)),
                "medium" => Ok(CssValue::AbsoluteSize(dtype::AbsoluteSize::Medium)),
                "large" => Ok(CssValue::AbsoluteSize(dtype::AbsoluteSize::Large)),
                "x-large" => Ok(CssValue::AbsoluteSize(dtype::AbsoluteSize::XLarge)),
                "xx-large" => Ok(CssValue::AbsoluteSize(dtype::AbsoluteSize::XXLarge)),
                _ => bail!("Invalid absolute size value: {:?}", v),
            },
            _ => bail!("Expected absolute size value but found: {:?}", v),
        },
        None => bail!("Expected absolute size value but found none"),
    }
}

// <relative-size> = smaller | larger
pub fn parse_relative_size_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    match values.next() {
        Some(v) => match &v {
            ComponentValue::PreservedToken(CssToken::Ident(size)) => match size.as_str() {
                "larger" => Ok(CssValue::RelativeSize(dtype::RelativeSize::Larger)),
                "smaller" => Ok(CssValue::RelativeSize(dtype::RelativeSize::Smaller)),
                _ => bail!("Invalid relative size value: {:?}", v),
            },
            _ => bail!("Expected relative size value but found: {:?}", v),
        },
        None => bail!("Expected relative size value but found none"),
    }
}

#[cfg(test)]
mod tests {
    use crate::renderer::css::tokenizer::NumericType;

    use super::*;

    #[test]
    fn test_parse_font_size() {
        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "small".to_string(),
        ))];
        assert_eq!(
            parse_font_size(&value).unwrap(),
            FontSizeProp {
                size: CssValue::AbsoluteSize(dtype::AbsoluteSize::Small)
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "medium".to_string(),
        ))];
        assert_eq!(
            parse_font_size(&value).unwrap(),
            FontSizeProp {
                size: CssValue::AbsoluteSize(dtype::AbsoluteSize::Medium)
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "large".to_string(),
        ))];
        assert_eq!(
            parse_font_size(&value).unwrap(),
            FontSizeProp {
                size: CssValue::AbsoluteSize(dtype::AbsoluteSize::Large)
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "larger".to_string(),
        ))];
        assert_eq!(
            parse_font_size(&value).unwrap(),
            FontSizeProp {
                size: CssValue::RelativeSize(dtype::RelativeSize::Larger)
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "smaller".to_string(),
        ))];
        assert_eq!(
            parse_font_size(&value).unwrap(),
            FontSizeProp {
                size: CssValue::RelativeSize(dtype::RelativeSize::Smaller)
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Dimension(
            NumericType::Number(12.0),
            "px".to_string(),
        ))];
        assert_eq!(
            parse_font_size(&value).unwrap(),
            FontSizeProp {
                size: CssValue::Length(
                    12.0,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)
                )
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Dimension(
            NumericType::Number(1.5),
            "em".to_string(),
        ))];
        assert_eq!(
            parse_font_size(&value).unwrap(),
            FontSizeProp {
                size: CssValue::Length(1.5, LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Em))
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Percentage(50.0))];
        assert_eq!(
            parse_font_size(&value).unwrap(),
            FontSizeProp {
                size: CssValue::Percentage(50.0,)
            }
        );
    }
}
