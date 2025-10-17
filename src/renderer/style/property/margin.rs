use std::{fmt, iter::Peekable};

use crate::{
    error::{Error, Result},
    renderer::{
        css::{cssom::ComponentValue, token::CssToken},
        style::{
            SpecifiedStyle,
            property::{
                AbsoluteLengthUnit, CssProperty, CssValue, LengthUnit, RelativeLengthUnit,
                font_size::{self, FontSizeProp},
                parse_length_percentage_type,
            },
        },
    },
};

/// https://developer.mozilla.org/en-US/docs/Web/CSS/margin
#[derive(Clone, Debug, PartialEq)]
pub struct MarginProp {
    pub top: CssValue,
    pub right: CssValue,
    pub bottom: CssValue,
    pub left: CssValue,
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

impl Default for MarginProp {
    fn default() -> Self {
        Self {
            top: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            right: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            bottom: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            left: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
        }
    }
}

impl CssProperty for MarginProp {
    // margin =
    //   <'margin-top'>{1,4}
    fn parse(values: &[ComponentValue]) -> Result<Self> {
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
            _ => Err(Error::CssProperty(format!(
                "Invalid margin declaration: {:?}",
                trbl
            ))),
        }
    }

    fn compute(
        &mut self,
        current_style: Option<&SpecifiedStyle>,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<&Self> {
        self.top = Self::compute_top(&self.top, current_style, viewport_width, viewport_height)?;
        self.right =
            Self::compute_top(&self.right, current_style, viewport_width, viewport_height)?;
        self.bottom =
            Self::compute_top(&self.bottom, current_style, viewport_width, viewport_height)?;
        self.left = Self::compute_top(&self.left, current_style, viewport_width, viewport_height)?;
        Ok(self)
    }
}

impl MarginProp {
    fn compute_top(
        value: &CssValue,
        current_style: Option<&SpecifiedStyle>,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<CssValue> {
        let current_font_size = current_style.and_then(|s| s.font_size.as_ref());
        let current_font_size = match current_font_size {
            Some(FontSizeProp {
                size: CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            }) => size,
            None => &font_size::MEDIUM,
            _ => {
                return Err(Error::CssProperty(format!(
                    "Invalid font-size value: {:?}",
                    current_font_size
                )));
            }
        };
        match &value {
            CssValue::Ident(v) => {
                if v != "auto" {
                    unimplemented!()
                }
                Ok(value.clone())
            }
            CssValue::Length(size, unit) => match unit {
                LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px) => Ok(value.clone()),
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Em) => Ok(CssValue::Length(
                    size * current_font_size,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                )),
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Vw) => Ok(CssValue::Length(
                    size * (viewport_width as f32) / 100.0,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                )),
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Vh) => Ok(CssValue::Length(
                    size * (viewport_height as f32) / 100.0,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                )),
                _ => unimplemented!("{:?} unit is not supported yet", unit),
            },
            CssValue::Percentage(_) => unimplemented!(),
            _ => Err(Error::CssProperty(format!(
                "Invalid margin value: {:?}",
                &value
            ))),
        }
    }
}

/// https://developer.mozilla.org/en-US/docs/Web/CSS/margin-block
#[derive(Clone, Debug, PartialEq)]
pub struct MarginBlockProp {
    pub start: CssValue,
    pub end: CssValue,
}

impl fmt::Display for MarginBlockProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.start, self.end)
    }
}

impl Default for MarginBlockProp {
    fn default() -> Self {
        Self {
            start: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            end: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
        }
    }
}

impl CssProperty for MarginBlockProp {
    // margin-block =
    //   <'margin-top'>{1,2}
    fn parse(values: &[ComponentValue]) -> Result<Self> {
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
                Ok(Self {
                    start: v.clone(),
                    end: v,
                })
            }
            2 => Ok(Self {
                start: start_end.first().unwrap().clone(),
                end: start_end.get(1).unwrap().clone(),
            }),
            _ => Err(Error::CssProperty(format!(
                "Invalid margin-block declaration: {:?}",
                start_end
            ))),
        }
    }

    fn compute(
        &mut self,
        current_style: Option<&SpecifiedStyle>,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<&Self> {
        self.start =
            Self::compute_top(&self.start, current_style, viewport_width, viewport_height)?;
        self.end = Self::compute_top(&self.end, current_style, viewport_width, viewport_height)?;
        Ok(self)
    }
}

impl MarginBlockProp {
    fn compute_top(
        value: &CssValue,
        current_style: Option<&SpecifiedStyle>,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<CssValue> {
        let current_font_size = current_style.and_then(|s| s.font_size.as_ref());
        let current_font_size = match current_font_size {
            Some(FontSizeProp {
                size: CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            }) => size,
            None => &font_size::MEDIUM,
            _ => {
                return Err(Error::CssProperty(format!(
                    "Invalid font-size value: {:?}",
                    current_font_size
                )));
            }
        };
        match &value {
            CssValue::Length(size, unit) => match unit {
                LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px) => Ok(value.clone()),
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Em) => Ok(CssValue::Length(
                    size * current_font_size,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                )),
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Vw) => Ok(CssValue::Length(
                    size * (viewport_width as f32) / 100.0,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                )),
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Vh) => Ok(CssValue::Length(
                    size * (viewport_height as f32) / 100.0,
                    LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                )),
                _ => unimplemented!(),
            },
            CssValue::Percentage(_) => unimplemented!(),
            _ => Err(Error::CssProperty(format!(
                "Invalid margin value: {:?}",
                &value
            ))),
        }
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
            "auto" => {
                values.next();
                Ok(CssValue::Ident("auto".to_string()))
            }
            _ => unimplemented!(),
        },
        _ => parse_length_percentage_type(values),
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
            MarginProp::parse(&value).unwrap(),
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
