use std::fmt;

use anyhow::{Ok, Result, bail};

use crate::renderer::{
    css::{cssom::ComponentValue, token::CssToken},
    style::{
        SpecifiedStyle,
        property::{
            AbsoluteLengthUnit, CssProperty, CssValue, LengthUnit, RelativeLengthUnit,
            font_size::{self, FontSizeProp},
            parse_length_percentage_type,
        },
    },
};

/// https://developer.mozilla.org/en-US/docs/Web/CSS/border-radius
#[derive(Clone, Debug, PartialEq)]
pub struct BorderRadiusProp {
    pub top_left: CssValue,
    pub top_right: CssValue,
    pub bottom_right: CssValue,
    pub bottom_left: CssValue,
}

impl fmt::Display for BorderRadiusProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.top_left, self.top_right, self.bottom_right, self.bottom_left
        )
    }
}

impl Default for BorderRadiusProp {
    fn default() -> Self {
        BorderRadiusProp {
            top_left: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            top_right: CssValue::Length(
                0.0,
                LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
            ),
            bottom_right: CssValue::Length(
                0.0,
                LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
            ),
            bottom_left: CssValue::Length(
                0.0,
                LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
            ),
        }
    }
}

impl CssProperty for BorderRadiusProp {
    // border-radius =
    //    <length-percentage [0,∞]>{1,4} [ / <length-percentage [0,∞]>{1,4} ]?
    fn parse(values: &[ComponentValue]) -> Result<Self> {
        let mut values = values.iter().cloned().peekable();
        let mut trbl = vec![];
        while values.peek().is_some() {
            while values
                .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
                .is_some()
            {}
            if values.peek() == Some(&ComponentValue::PreservedToken(CssToken::Delim('/'))) {
                unimplemented!()
            }
            if values.peek().is_some() {
                trbl.push(parse_length_percentage_type(&mut values)?);
            }
        }
        match trbl.len() {
            1 => {
                let v = trbl.first().unwrap().clone();
                Ok(Self {
                    top_left: v.clone(),
                    top_right: v.clone(),
                    bottom_right: v.clone(),
                    bottom_left: v,
                })
            }
            2 => {
                let top_left = trbl.first().unwrap().clone();
                let top_right = trbl.get(1).unwrap().clone();
                Ok(Self {
                    top_left: top_left.clone(),
                    top_right: top_right.clone(),
                    bottom_right: top_left,
                    bottom_left: top_right,
                })
            }
            3 => {
                let top_right = trbl.get(1).unwrap().clone();
                Ok(Self {
                    top_left: trbl.first().unwrap().clone(),
                    top_right: top_right.clone(),
                    bottom_right: trbl.get(2).unwrap().clone(),
                    bottom_left: top_right,
                })
            }
            4 => Ok(Self {
                top_left: trbl.first().unwrap().clone(),
                top_right: trbl.get(1).unwrap().clone(),
                bottom_right: trbl.get(2).unwrap().clone(),
                bottom_left: trbl.get(3).unwrap().clone(),
            }),
            _ => bail!("Invalid border-radius value: {:?}", trbl),
        }
    }

    fn compute(&mut self, current_style: Option<&SpecifiedStyle>) -> Result<&Self> {
        self.top_left = Self::compute_top(&self.top_left, current_style)?;
        self.top_right = Self::compute_top(&self.top_right, current_style)?;
        self.bottom_right = Self::compute_top(&self.bottom_right, current_style)?;
        self.bottom_left = Self::compute_top(&self.bottom_left, current_style)?;
        Ok(self)
    }
}

impl BorderRadiusProp {
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

    pub fn to_px(&self) -> Result<(f64, f64, f64, f64)> {
        Ok((
            self.top_left.to_px()? as f64,
            self.top_right.to_px()? as f64,
            self.bottom_right.to_px()? as f64,
            self.bottom_left.to_px()? as f64,
        ))
    }
}
