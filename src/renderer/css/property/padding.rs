use std::fmt;
use std::iter::Peekable;

use anyhow::{bail, Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::dtype::{
    parse_length_percentage_type, AbsoluteLengthUnit, CssValue, LengthUnit, RelativeLengthUnit,
};
use crate::renderer::css::property::font_size::{self, FontSizeProp};
use crate::renderer::css::tokenizer::CssToken;

#[derive(Clone, Debug, PartialEq)]
pub struct PaddingProp {
    pub top: CssValue,
    pub right: CssValue,
    pub bottom: CssValue,
    pub left: CssValue,
}

impl fmt::Display for PaddingProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.top, self.right, self.bottom, self.left
        )
    }
}

impl PaddingProp {
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
            _ => bail!("Invalid padding value: {:?}", &value),
        }
    }
}

// padding =
//   <'padding-top'>{1,4}
pub fn parse_padding(values: &[ComponentValue]) -> Result<PaddingProp> {
    let mut values = values.iter().cloned().peekable();
    let mut trbl = vec![];
    while values.peek().is_some() {
        while values
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {}
        if values.peek().is_some() {
            trbl.push(parse_padding_top_type(&mut values)?);
        }
    }
    match trbl.len() {
        1 => {
            let v = trbl.first().unwrap().clone();
            Ok(PaddingProp {
                top: v.clone(),
                right: v.clone(),
                bottom: v.clone(),
                left: v,
            })
        }
        2 => {
            let top = trbl.first().unwrap().clone();
            let right = trbl.get(1).unwrap().clone();
            Ok(PaddingProp {
                top: top.clone(),
                right: right.clone(),
                bottom: top,
                left: right,
            })
        }
        3 => {
            let right = trbl.get(1).unwrap().clone();
            Ok(PaddingProp {
                top: trbl.first().unwrap().clone(),
                right: right.clone(),
                bottom: trbl.get(2).unwrap().clone(),
                left: right,
            })
        }
        4 => Ok(PaddingProp {
            top: trbl.first().unwrap().clone(),
            right: trbl.get(1).unwrap().clone(),
            bottom: trbl.get(2).unwrap().clone(),
            left: trbl.get(3).unwrap().clone(),
        }),
        _ => bail!("Invalid margin declaration: {:?}", values),
    }
}

// <padding-top> =
//   <length-percentage [0,∞]>
fn parse_padding_top_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}
    parse_length_percentage_type(values)
}
