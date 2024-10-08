use std::fmt;
use std::iter::Peekable;

use anyhow::{bail, Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::dtype::{
    parse_length_percentage_type, AbsoluteLengthUnit, CssValue, LengthUnit, RelativeLengthUnit,
};
use crate::renderer::css::property::font_size;
use crate::renderer::css::token::CssToken;

use super::font_size::FontSizeProp;

#[derive(Clone, Debug, PartialEq)]
pub struct MarginProp {
    pub top: CssValue,
    pub right: CssValue,
    pub bottom: CssValue,
    pub left: CssValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarginBlockProp {
    pub start: CssValue,
    pub end: CssValue,
}

impl fmt::Display for MarginProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.top, self.right, self.bottom, self.left
        )
    }
}

impl fmt::Display for MarginBlockProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.start, self.end)
    }
}

impl Default for MarginProp {
    fn default() -> Self {
        MarginProp {
            top: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            right: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            bottom: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            left: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
        }
    }
}

impl MarginProp {
    pub fn compute(&mut self, current_font_size: Option<&FontSizeProp>) -> Result<&Self> {
        self.top = Self::compute_top(&self.top, current_font_size)?;
        self.right = Self::compute_top(&self.right, current_font_size)?;
        self.bottom = Self::compute_top(&self.bottom, current_font_size)?;
        self.left = Self::compute_top(&self.left, current_font_size)?;
        Ok(self)
    }

    fn compute_top(value: &CssValue, current_font_size: Option<&FontSizeProp>) -> Result<CssValue> {
        let current_font_size = match current_font_size {
            Some(FontSizeProp {
                size: CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            }) => size,
            None => &font_size::MEDIUM,
            _ => bail!("Invalid font-size value: {:?}", current_font_size),
        };
        match &value {
            CssValue::Length(size, unit) => match unit {
                LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px) => Ok(value.clone()),
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Em) => Ok(CssValue::Length(
                    size * current_font_size,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                )),
                _ => unimplemented!(),
            },
            CssValue::Percentage(_) => unimplemented!(),
            _ => bail!("Invalid margin value: {:?}", &value),
        }
    }
}

impl MarginBlockProp {
    pub fn compute(&mut self, current_font_size: Option<&FontSizeProp>) -> Result<&Self> {
        self.start = Self::compute_top(&self.start, current_font_size)?;
        self.end = Self::compute_top(&self.end, current_font_size)?;
        Ok(self)
    }

    fn compute_top(value: &CssValue, current_font_size: Option<&FontSizeProp>) -> Result<CssValue> {
        let current_font_size = match current_font_size {
            Some(FontSizeProp {
                size: CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            }) => size,
            None => &font_size::MEDIUM,
            _ => bail!("Invalid font-size value: {:?}", current_font_size),
        };
        match &value {
            CssValue::Length(size, unit) => match unit {
                LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px) => Ok(value.clone()),
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Em) => Ok(CssValue::Length(
                    size * current_font_size,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                )),
                _ => unimplemented!(),
            },
            CssValue::Percentage(_) => unimplemented!(),
            _ => bail!("Invalid margin value: {:?}", &value),
        }
    }
}

// margin =
//   <'margin-top'>{1,4}
pub fn parse_margin(values: &[ComponentValue]) -> Result<MarginProp> {
    let mut values = values.iter().cloned().peekable();
    let mut trbl = vec![];
    while values.peek().is_some() {
        while values
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {}
        if values.peek().is_some() {
            trbl.push(parse_margin_top_type(&mut values)?);
        }
    }
    match trbl.len() {
        1 => {
            let v = trbl.first().unwrap().clone();
            Ok(MarginProp {
                top: v.clone(),
                right: v.clone(),
                bottom: v.clone(),
                left: v,
            })
        }
        2 => {
            let top = trbl.first().unwrap().clone();
            let right = trbl.get(1).unwrap().clone();
            Ok(MarginProp {
                top: top.clone(),
                right: right.clone(),
                bottom: top,
                left: right,
            })
        }
        3 => {
            let right = trbl.get(1).unwrap().clone();
            Ok(MarginProp {
                top: trbl.first().unwrap().clone(),
                right: right.clone(),
                bottom: trbl.get(2).unwrap().clone(),
                left: right,
            })
        }
        4 => Ok(MarginProp {
            top: trbl.first().unwrap().clone(),
            right: trbl.get(1).unwrap().clone(),
            bottom: trbl.get(2).unwrap().clone(),
            left: trbl.get(3).unwrap().clone(),
        }),
        _ => bail!("Invalid margin declaration: {:?}", values),
    }
}

// <margin-top> =
//   <length-percentage>  |
//   auto                 |
//   <anchor-size()>
fn parse_margin_top_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}
    match values.peek() {
        Some(ComponentValue::PreservedToken(CssToken::Ident(size))) => match size.as_str() {
            "auto" => unimplemented!(),
            _ => unimplemented!(),
        },
        _ => parse_length_percentage_type(values),
    }
}

// margin-block =
//   <'margin-top'>{1,2}
pub fn parse_margin_block(values: &[ComponentValue]) -> Result<MarginBlockProp> {
    let mut values = values.iter().cloned().peekable();
    let mut start_end = vec![];
    while values.peek().is_some() {
        while values
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {}
        if values.peek().is_some() {
            start_end.push(parse_margin_top_type(&mut values)?);
        }
    }
    match start_end.len() {
        1 => {
            let v = start_end.first().unwrap().clone();
            Ok(MarginBlockProp {
                start: v.clone(),
                end: v,
            })
        }
        2 => Ok(MarginBlockProp {
            start: start_end.first().unwrap().clone(),
            end: start_end.get(1).unwrap().clone(),
        }),
        _ => bail!("Invalid margin-block declaration: {:?}", start_end),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::css::token::NumericType;

    #[test]
    fn parse_valid_margin() {
        let value = vec![
            ComponentValue::PreservedToken(CssToken::Dimension(
                NumericType::Number(10.0),
                "px".to_string(),
            )),
            ComponentValue::PreservedToken(CssToken::Dimension(
                NumericType::Number(20.0),
                "px".to_string(),
            )),
            ComponentValue::PreservedToken(CssToken::Dimension(
                NumericType::Number(30.0),
                "px".to_string(),
            )),
            ComponentValue::PreservedToken(CssToken::Dimension(
                NumericType::Number(40.0),
                "px".to_string(),
            )),
        ];
        assert_eq!(
            parse_margin(&value).unwrap(),
            MarginProp {
                top: CssValue::Length(10.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
                right: CssValue::Length(
                    20.0,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)
                ),
                bottom: CssValue::Length(
                    30.0,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)
                ),
                left: CssValue::Length(
                    40.0,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)
                ),
            }
        );
    }
}
