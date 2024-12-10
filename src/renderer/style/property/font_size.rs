use std::fmt;
use std::iter::Peekable;

use anyhow::{bail, Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::token::CssToken;
use crate::renderer::style::property::{
    parse_length_percentage_type, AbsoluteLengthUnit, AbsoluteSize, CssProperty, CssValue,
    LengthUnit, RelativeLengthUnit, RelativeSize,
};
use crate::renderer::style::style_model::SpecifiedValues;

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

impl Default for FontSizeProp {
    fn default() -> Self {
        Self {
            size: CssValue::AbsoluteSize(AbsoluteSize::Medium),
        }
    }
}

impl CssProperty for FontSizeProp {
    // font-size =
    //   <absolute-size>            |
    //   <relative-size>            |
    //   <length-percentage [0,∞]>  |
    //   math
    fn parse(values: &[ComponentValue]) -> Result<Self> {
        let mut values = values.iter().cloned().peekable();
        if let Some(ComponentValue::PreservedToken(CssToken::Ident(size))) = values.peek() {
            match size.as_str() {
                "xx-small" | "x-small" | "small" | "medium" | "large" | "x-large" | "xx-large" => {
                    Ok(Self {
                        size: parse_absolute_size_type(&mut values)?,
                    })
                }
                "larger" | "smaller" => Ok(Self {
                    size: parse_relative_size_type(&mut values)?,
                }),
                _ => bail!(
                    "Expected absolute or relative size value but found: {:?}",
                    values
                ),
            }
        } else {
            Ok(Self {
                size: parse_length_percentage_type(&mut values)?,
            })
        }
    }

    fn compute(&mut self, parent_style: Option<&SpecifiedValues>) -> Result<&Self> {
        let parent_px = match parent_style.and_then(|s| s.font_size.as_ref()) {
            Some(FontSizeProp {
                size: CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            }) => *size,
            _ => MEDIUM,
        };

        match &self.size {
            CssValue::AbsoluteSize(size) => match size {
                AbsoluteSize::Small => {
                    self.size = CssValue::Length(
                        SMALL,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    );
                }
                AbsoluteSize::Medium => {
                    self.size = CssValue::Length(
                        MEDIUM,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    );
                }
                AbsoluteSize::Large => {
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

// <absolute-size> = xx-small | x-small | small | medium | large | x-large | xx-large | xxx-large
pub fn parse_absolute_size_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    match values.next() {
        Some(v) => match &v {
            ComponentValue::PreservedToken(CssToken::Ident(size)) => match size.as_str() {
                "xx-small" => Ok(CssValue::AbsoluteSize(AbsoluteSize::XXSmall)),
                "x-small" => Ok(CssValue::AbsoluteSize(AbsoluteSize::XSmall)),
                "small" => Ok(CssValue::AbsoluteSize(AbsoluteSize::Small)),
                "medium" => Ok(CssValue::AbsoluteSize(AbsoluteSize::Medium)),
                "large" => Ok(CssValue::AbsoluteSize(AbsoluteSize::Large)),
                "x-large" => Ok(CssValue::AbsoluteSize(AbsoluteSize::XLarge)),
                "xx-large" => Ok(CssValue::AbsoluteSize(AbsoluteSize::XXLarge)),
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
                "larger" => Ok(CssValue::RelativeSize(RelativeSize::Larger)),
                "smaller" => Ok(CssValue::RelativeSize(RelativeSize::Smaller)),
                _ => bail!("Invalid relative size value: {:?}", v),
            },
            _ => bail!("Expected relative size value but found: {:?}", v),
        },
        None => bail!("Expected relative size value but found none"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::css::token::NumericType;

    #[test]
    fn parse_size() {
        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "small".to_string(),
        ))];
        assert_eq!(
            FontSizeProp::parse(&value).unwrap(),
            FontSizeProp {
                size: CssValue::AbsoluteSize(AbsoluteSize::Small)
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "medium".to_string(),
        ))];
        assert_eq!(
            FontSizeProp::parse(&value).unwrap(),
            FontSizeProp {
                size: CssValue::AbsoluteSize(AbsoluteSize::Medium)
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "large".to_string(),
        ))];
        assert_eq!(
            FontSizeProp::parse(&value).unwrap(),
            FontSizeProp {
                size: CssValue::AbsoluteSize(AbsoluteSize::Large)
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "larger".to_string(),
        ))];
        assert_eq!(
            FontSizeProp::parse(&value).unwrap(),
            FontSizeProp {
                size: CssValue::RelativeSize(RelativeSize::Larger)
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "smaller".to_string(),
        ))];
        assert_eq!(
            FontSizeProp::parse(&value).unwrap(),
            FontSizeProp {
                size: CssValue::RelativeSize(RelativeSize::Smaller)
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Dimension(
            NumericType::Number(12.0),
            "px".to_string(),
        ))];
        assert_eq!(
            FontSizeProp::parse(&value).unwrap(),
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
            FontSizeProp::parse(&value).unwrap(),
            FontSizeProp {
                size: CssValue::Length(1.5, LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Em))
            }
        );

        let value = vec![ComponentValue::PreservedToken(CssToken::Percentage(50.0))];
        assert_eq!(
            FontSizeProp::parse(&value).unwrap(),
            FontSizeProp {
                size: CssValue::Percentage(50.0,)
            }
        );
    }
}
