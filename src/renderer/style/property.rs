mod border;
mod border_radius;
mod color;
mod display;
mod font_family;
mod font_size;
mod font_weight;
mod height;
mod margin;
mod padding;
mod text_decoration;
mod width;

pub use self::{
    border::BorderProp,
    border_radius::BorderRadiusProp,
    color::{BackGroundColorProp, ColorProp},
    display::{DisplayBox, DisplayInside, DisplayOutside, DisplayProp},
    font_family::FontFamilyProp,
    font_size::FontSizeProp,
    font_weight::FontWeightProp,
    height::HeightProp,
    margin::{MarginBlockProp, MarginProp},
    padding::PaddingProp,
    text_decoration::TextDecorationProp,
    width::WidthProp,
};

use std::{fmt, iter::Peekable};

use anyhow::{Result, bail};

use crate::renderer::{
    css::{
        cssom::ComponentValue,
        token::{CssToken, NumericType},
    },
    style::{SpecifiedStyle, property::color::rgb_to_name},
};

pub trait CssProperty {
    fn parse(values: &[ComponentValue]) -> Result<Self>
    where
        Self: Sized;
    fn compute(&mut self, current_style: Option<&SpecifiedStyle>) -> Result<&Self>
    where
        Self: Sized;
}

/// https://www.w3.org/TR/css-values-3/
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum CssValue {
    Ident(String),
    String(String),
    Integer(i32),
    Number(f32),
    Dimension(f32, String),
    Percentage(f32),
    Length(f32, LengthUnit),
    Color { r: u8, g: u8, b: u8, a: f32 },
    HexColor(String),
    AbsoluteSize(AbsoluteSize),
    RelativeSize(RelativeSize),
}

impl fmt::Display for CssValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CssValue::Ident(ident) => write!(f, "{}", ident),
            CssValue::String(string) => write!(f, "{}", string),
            CssValue::Integer(integer) => write!(f, "{}", integer),
            CssValue::Number(float) => write!(f, "{}", float),
            CssValue::Dimension(value, unit) => write!(f, "{}{}", value, unit),
            CssValue::Percentage(value) => write!(f, "{}%", value),
            CssValue::Length(value, unit) => write!(f, "{}{}", value, unit),
            CssValue::Color { r, g, b, a } => {
                if *a == 1.0 {
                    let name = rgb_to_name(*r, *g, *b);
                    if name.is_some() {
                        write!(f, "{}", name.unwrap())
                    } else {
                        write!(f, "rgb({}, {}, {})", r, g, b)
                    }
                } else {
                    write!(f, "rgba({}, {}, {}, {})", r, g, b, a)
                }
            }
            CssValue::HexColor(color) => write!(f, "{}", color),
            CssValue::AbsoluteSize(size) => write!(f, "{:?}", size),
            CssValue::RelativeSize(size) => write!(f, "{:?}", size),
        }
    }
}

impl CssValue {
    /// Converts the length value to pixels, if its unit is `px`.
    pub fn to_px(&self) -> Result<f32> {
        if let CssValue::Length(value, unit) = self {
            if unit == &LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px) {
                return Ok(*value);
            }
            bail!("Expected px unit but found: {:?}", unit);
        } else {
            bail!("Expected length value but found: {:?}", self);
        }
    }

    /// Converts the color value to rgba. R: 0-255, G: 0-255, B: 0-255, A: 0.0-1.0
    pub fn to_rgba(&self) -> Result<(u8, u8, u8, f32)> {
        if let CssValue::Color { r, g, b, a } = self {
            Ok((*r, *g, *b, *a))
        } else {
            bail!("Expected color value but found: {:?}", self);
        }
    }

    /// Converts the ident or string value to a string name.
    pub fn to_name(&self) -> Result<String> {
        if let CssValue::Ident(s) | CssValue::String(s) = self {
            Ok(s.to_owned())
        } else {
            bail!("Expected string value but found: {:?}", self);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LengthUnit {
    RelativeLengthUnit(RelativeLengthUnit),
    AbsoluteLengthUnit(AbsoluteLengthUnit),
}

impl fmt::Display for LengthUnit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LengthUnit::RelativeLengthUnit(unit) => write!(f, "{:?}", unit),
            LengthUnit::AbsoluteLengthUnit(unit) => write!(f, "{:?}", unit),
        }
    }
}

/// https://www.w3.org/TR/css-values-3/#relative-lengths
#[derive(Clone, Debug, PartialEq)]
pub enum RelativeLengthUnit {
    Em,
    Ex,
    Ch,
    Rem,
    Vw,
    Vh,
    Vmin,
    Vmax,
}

/// https://www.w3.org/TR/css-values-3/#absolute-lengths
#[derive(Clone, Debug, PartialEq)]
pub enum AbsoluteLengthUnit {
    Cm,
    Mm,
    Q,
    In,
    Pc,
    Pt,
    Px,
}

/// https://www.w3.org/TR/css-fonts-3/#absolute-size-value
#[derive(Clone, Debug, PartialEq)]
pub enum AbsoluteSize {
    XXSmall,
    XSmall,
    Small,
    Medium,
    Large,
    XLarge,
    XXLarge,
}

/// https://www.w3.org/TR/css-fonts-3/#relative-size-value
#[derive(Clone, Debug, PartialEq)]
pub enum RelativeSize {
    Larger,
    Smaller,
}

// <length-percentage> =
//   <length>      |
//   <percentage>
pub fn parse_length_percentage_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    match values.peek() {
        Some(v) => match v {
            ComponentValue::PreservedToken(CssToken::Dimension(..) | CssToken::Number(..)) => {
                parse_length_type(values)
            }
            ComponentValue::PreservedToken(CssToken::Percentage(..)) => {
                parse_percentage_type(values)
            }
            _ => bail!(
                "Expected length or percentage value but found: {:?}",
                values.peek()
            ),
        },
        _ => bail!("Expected length or percentage value but found none"),
    }
}

pub fn parse_length_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    match values.next() {
        Some(v) => match v {
            ComponentValue::PreservedToken(CssToken::Dimension(size, unit)) => {
                let size = match size {
                    NumericType::Integer(n) => n as f32,
                    NumericType::Number(n) => n,
                };
                match unit.as_str() {
                    "cm" => Ok(CssValue::Length(
                        size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Cm),
                    )),
                    "mm" => Ok(CssValue::Length(
                        size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Mm),
                    )),
                    "q" => Ok(CssValue::Length(
                        size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Q),
                    )),
                    "in" => Ok(CssValue::Length(
                        size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::In),
                    )),
                    "pc" => Ok(CssValue::Length(
                        size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Pc),
                    )),
                    "pt" => Ok(CssValue::Length(
                        size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Pt),
                    )),
                    "px" => Ok(CssValue::Length(
                        size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    )),
                    "em" => Ok(CssValue::Length(
                        size,
                        LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Em),
                    )),
                    "ex" => Ok(CssValue::Length(
                        size,
                        LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Ex),
                    )),
                    "ch" => Ok(CssValue::Length(
                        size,
                        LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Ch),
                    )),
                    "rem" => Ok(CssValue::Length(
                        size,
                        LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Rem),
                    )),
                    "vw" => Ok(CssValue::Length(
                        size,
                        LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Vw),
                    )),
                    "vh" => Ok(CssValue::Length(
                        size,
                        LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Vh),
                    )),
                    "vmin" => Ok(CssValue::Length(
                        size,
                        LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Vmin),
                    )),
                    "vmax" => Ok(CssValue::Length(
                        size,
                        LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Vmax),
                    )),
                    _ => unimplemented!(),
                }
            }
            ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))) => Ok(
                CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            ),
            _ => bail!("Expected length value but found: {:?}", v),
        },
        None => bail!("Expected length value but found none"),
    }
}

pub fn parse_percentage_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    match values.next() {
        Some(v) => match v {
            ComponentValue::PreservedToken(CssToken::Percentage(size)) => {
                Ok(CssValue::Percentage(size))
            }
            _ => bail!("Expected percentage value but found: {:?}", v),
        },
        None => bail!("Expected percentage value but found none"),
    }
}
