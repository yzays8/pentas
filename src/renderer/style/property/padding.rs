use std::fmt;
use std::iter::Peekable;

use anyhow::{bail, Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::token::CssToken;
use crate::renderer::layout::Edge;
use crate::renderer::style::property::font_size::{self, FontSizeProp};
use crate::renderer::style::property::{
    parse_length_percentage_type, AbsoluteLengthUnit, CssProperty, CssValue, LengthUnit,
    RelativeLengthUnit,
};
use crate::renderer::style::SpecifiedStyle;

/// https://developer.mozilla.org/en-US/docs/Web/CSS/padding
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

impl Default for PaddingProp {
    fn default() -> Self {
        Self {
            top: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            right: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            bottom: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            left: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
        }
    }
}

impl CssProperty for PaddingProp {
    // padding =
    //   <'padding-top'>{1,4}
    fn parse(values: &[ComponentValue]) -> Result<Self> {
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
                Ok(Self {
                    top: v.clone(),
                    right: v.clone(),
                    bottom: v.clone(),
                    left: v,
                })
            }
            2 => {
                let top = trbl.first().unwrap().clone();
                let right = trbl.get(1).unwrap().clone();
                Ok(Self {
                    top: top.clone(),
                    right: right.clone(),
                    bottom: top,
                    left: right,
                })
            }
            3 => {
                let right = trbl.get(1).unwrap().clone();
                Ok(Self {
                    top: trbl.first().unwrap().clone(),
                    right: right.clone(),
                    bottom: trbl.get(2).unwrap().clone(),
                    left: right,
                })
            }
            4 => Ok(Self {
                top: trbl.first().unwrap().clone(),
                right: trbl.get(1).unwrap().clone(),
                bottom: trbl.get(2).unwrap().clone(),
                left: trbl.get(3).unwrap().clone(),
            }),
            _ => bail!("Invalid margin declaration: {:?}", values),
        }
    }

    fn compute(&mut self, current_style: Option<&SpecifiedStyle>) -> Result<&Self> {
        self.top = Self::compute_top(&self.top, current_style)?;
        self.right = Self::compute_top(&self.right, current_style)?;
        self.bottom = Self::compute_top(&self.bottom, current_style)?;
        self.left = Self::compute_top(&self.left, current_style)?;
        Ok(self)
    }
}

impl PaddingProp {
    fn compute_top(value: &CssValue, current_style: Option<&SpecifiedStyle>) -> Result<CssValue> {
        let current_font_size = current_style.and_then(|s| s.font_size.as_ref());
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

    pub fn to_px(&self) -> Result<Edge> {
        Ok(Edge {
            top: self.top.to_px()?,
            right: self.right.to_px()?,
            bottom: self.bottom.to_px()?,
            left: self.left.to_px()?,
        })
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
