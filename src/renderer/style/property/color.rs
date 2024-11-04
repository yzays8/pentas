use std::fmt;
use std::iter::Peekable;

use anyhow::{anyhow, bail, ensure, Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::token::{CssToken, NumericType};
use crate::renderer::style::value_type::CssValue;

#[derive(Clone, Debug, PartialEq)]
pub struct ColorProp {
    pub value: CssValue,
}

impl fmt::Display for ColorProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Default for ColorProp {
    fn default() -> Self {
        ColorProp {
            value: CssValue::Color {
                r: 0,
                g: 0,
                b: 0,
                a: 1.0,
            },
        }
    }
}

impl ColorProp {
    // color =
    //   <color>
    pub fn parse(values: &[ComponentValue]) -> Result<Self> {
        Ok(Self {
            value: parse_color_type(&mut values.iter().cloned().peekable())?,
        })
    }

    pub fn compute(&mut self, current_color: Option<&Self>) -> Result<&Self> {
        match &self.value {
            CssValue::Ident(name) => match name.to_ascii_lowercase().as_str() {
                "currentcolor" => {
                    if let Some(curr) = current_color {
                        self.value = curr.value.clone();
                    } else {
                        self.value = CssValue::Color {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 1.0,
                        };
                    }
                }
                "transparent" => {
                    self.value = CssValue::Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0.0,
                    };
                }
                _ => {
                    if let Some((r, g, b)) = name_to_rgb(name) {
                        self.value = CssValue::Color { r, g, b, a: 1.0 };
                    } else {
                        bail!("Failed to compute color: {}", name);
                    }
                }
            },
            CssValue::HexColor(hex) => {
                let (r, g, b, a) = hex_to_rgba(hex)?;
                self.value = CssValue::Color {
                    r,
                    g,
                    b,
                    a: a as f32 / 255.0,
                };
            }
            CssValue::Color { .. } => {}
            _ => bail!("Failed to compute color: {:?}", self.value),
        }
        Ok(self)
    }
}

// <color> =
//   <color-base>    |
//   currentColor    |
//   <system-color>
pub fn parse_color_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}

    match values.peek() {
        Some(ComponentValue::PreservedToken(CssToken::Ident(v))) => {
            if v.eq_ignore_ascii_case("currentColor") {
                values.next();
                Ok(CssValue::Ident("currentColor".to_string()))
            } else {
                // todo: parse system color
                Ok(parse_color_base_type(values)?)
            }
        }
        _ => Ok(parse_color_base_type(values)?),
    }
}

// <color-base> =
//   <hex-color>       |
//   <color-function>  |
//   <named-color>     |
//   transparent
fn parse_color_base_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}
    match values.peek() {
        Some(v) => match v {
            ComponentValue::PreservedToken(CssToken::Ident(v)) => {
                if v.eq_ignore_ascii_case("transparent") {
                    values.next();
                    Ok(CssValue::Ident("transparent".to_string()))
                } else {
                    Ok(parse_named_color_type(values)?)
                }
            }
            ComponentValue::PreservedToken(CssToken::Hash(..)) => Ok(parse_hex_color_type(values)?),
            ComponentValue::Function { .. } => Ok(parse_color_function_type(values)?),
            _ => bail!("Invalid color value: {:?}", v),
        },
        None => bail!("Expected color value but found nothing"),
    }
}

fn parse_named_color_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}
    match values.next() {
        Some(v) => match v {
            ComponentValue::PreservedToken(CssToken::Ident(v)) => {
                match v.to_ascii_lowercase().as_str() {
                    "black" => Ok(CssValue::Ident("black".to_string())),
                    "gray" => Ok(CssValue::Ident("gray".to_string())),
                    "white" => Ok(CssValue::Ident("white".to_string())),
                    "red" => Ok(CssValue::Ident("red".to_string())),
                    "purple" => Ok(CssValue::Ident("purple".to_string())),
                    "blueviolet" => Ok(CssValue::Ident("blueviolet".to_string())),
                    "green" => Ok(CssValue::Ident("green".to_string())),
                    "yellowgreen" => Ok(CssValue::Ident("yellowgreen".to_string())),
                    "yellow" => Ok(CssValue::Ident("yellow".to_string())),
                    "blue" => Ok(CssValue::Ident("blue".to_string())),
                    "aqua" => Ok(CssValue::Ident("aqua".to_string())),
                    "orange" => Ok(CssValue::Ident("orange".to_string())),
                    "brown" => Ok(CssValue::Ident("brown".to_string())),
                    _ => unimplemented!(),
                }
            }
            _ => bail!("Invalid color value: {:?}", v),
        },
        None => bail!("Expected color value but found nothing"),
    }
}

fn parse_hex_color_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}
    match values.next() {
        Some(v) => match v {
            ComponentValue::PreservedToken(CssToken::Hash(v, _)) => match v.len() {
                3 => {
                    let r = u8::from_str_radix(&v[0..1].repeat(2), 16)?;
                    let g = u8::from_str_radix(&v[1..2].repeat(2), 16)?;
                    let b = u8::from_str_radix(&v[2..3].repeat(2), 16)?;
                    Ok(CssValue::HexColor(format!("{:02x}{:02x}{:02x}", r, g, b)))
                }
                4 => {
                    let r = u8::from_str_radix(&v[0..1].repeat(2), 16)?;
                    let g = u8::from_str_radix(&v[1..2].repeat(2), 16)?;
                    let b = u8::from_str_radix(&v[2..3].repeat(2), 16)?;
                    let a = u8::from_str_radix(&v[3..4].repeat(2), 16)?;
                    Ok(CssValue::HexColor(format!(
                        "{:02x}{:02x}{:02x}{:02x}",
                        r, g, b, a
                    )))
                }
                6 => {
                    let r = u8::from_str_radix(&v[0..2], 16)?;
                    let g = u8::from_str_radix(&v[2..4], 16)?;
                    let b = u8::from_str_radix(&v[4..6], 16)?;
                    Ok(CssValue::HexColor(format!("{:02x}{:02x}{:02x}", r, g, b)))
                }
                8 => {
                    let r = u8::from_str_radix(&v[0..2], 16)?;
                    let g = u8::from_str_radix(&v[2..4], 16)?;
                    let b = u8::from_str_radix(&v[4..6], 16)?;
                    let a = u8::from_str_radix(&v[6..8], 16)?;
                    Ok(CssValue::HexColor(format!(
                        "{:02x}{:02x}{:02x}{:02x}",
                        r, g, b, a
                    )))
                }
                _ => bail!("Invalid hex color: {}", v),
            },
            _ => bail!("Invalid color value: {:?}", v),
        },
        None => bail!("Expected color value but found nothing"),
    }
}

// <color-function> =
//   <rgb()>    |
//   <rgba()>   |
//   <hsl()>    |
//   <hsla()>   |
//   <hwb()>    |
//   <lab()>    |
//   <lch()>    |
//   <oklab()>  |
//   <oklch()>  |
//   <color()>
fn parse_color_function_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}
    match values.next() {
        Some(v) => match v {
            ComponentValue::Function { name, values } => match name.as_str() {
                "rgb" | "rgba" => {
                    let mut args = values.iter().cloned().peekable();
                    parse_color_rgb_function_type(&mut args)
                }
                _ => unimplemented!(),
            },
            _ => bail!("Invalid color value: {:?}", v),
        },
        None => bail!("Expected color value but found none"),
    }
}

// rgb() = [ <legacy-rgb-syntax> | <modern-rgb-syntax> ]
// rgba() = [ <legacy-rgba-syntax> | <modern-rgba-syntax> ]
// <legacy-rgb-syntax> =   rgb( <percentage>#{3} , <alpha-value>? ) |
//                   rgb( <number>#{3} , <alpha-value>? )
// <legacy-rgba-syntax> = rgba( <percentage>#{3} , <alpha-value>? ) |
//                   rgba( <number>#{3} , <alpha-value>? )
// <modern-rgb-syntax> = rgb(
//   [ <number> | <percentage> | none]{3}
//   [ / [<alpha-value> | none] ]?  )
// <modern-rgba-syntax> = rgba(
//   [ <number> | <percentage> | none]{3}
//   [ / [<alpha-value> | none] ]?  )
/// https://www.w3.org/TR/css-color-4/#rgb-functions
fn parse_color_rgb_function_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    let mut is_legacy_syntax = false;
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}

    let r = values
        .next_if(|v| {
            matches!(
                v,
                ComponentValue::PreservedToken(CssToken::Number(
                    NumericType::Integer(_) | NumericType::Number(_)
                ))
            )
        })
        .ok_or_else(|| anyhow!("Invalid rgb function"))?;
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}
    if values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Comma))
        .is_some()
    {
        is_legacy_syntax = true;
        while values
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {}
    }

    let g = values
        .next_if(|v| {
            matches!(
                v,
                ComponentValue::PreservedToken(CssToken::Number(
                    NumericType::Integer(_) | NumericType::Number(_)
                ))
            )
        })
        .ok_or_else(|| anyhow!("Invalid rgb function"))?;
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}
    if values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Comma))
        .is_some()
    {
        if !is_legacy_syntax {
            bail!("Invalid rgb function");
        }
        while values
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {}
    }

    let b = values
        .next_if(|v| {
            matches!(
                v,
                ComponentValue::PreservedToken(CssToken::Number(
                    NumericType::Integer(_) | NumericType::Number(_)
                ))
            )
        })
        .ok_or_else(|| anyhow!("Invalid rgb function"))?;
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}

    let is_separator_present = if is_legacy_syntax {
        if values
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Comma))
            .is_some()
        {
            while values
                .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
                .is_some()
            {}
            true
        } else {
            false
        }
    } else if values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Delim('/')))
        .is_some()
    {
        while values
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {}
        true
    } else {
        false
    };

    let a = match values.next() {
        Some(ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(v)))) => {
            ensure!(
                (0.0..=1.0).contains(&v),
                "Invalid value for alpha in rgb function"
            );
            v
        }
        Some(_) => bail!("Invalid rgb function"),
        None => {
            if is_separator_present {
                bail!("Invalid rgb function");
            }
            1.0
        }
    };

    let r = match r {
        ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(r))) => {
            ensure!(
                (0..=255).contains(&r),
                "Invalid value for red in rgb function"
            );
            r as u8
        }
        ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(r))) => {
            ensure!(
                (0.0..=255.0).contains(&r),
                "Invalid value for red in rgb function"
            );
            r.round() as u8
        }
        _ => bail!("Invalid rgb function"),
    };
    let g = match g {
        ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(g))) => {
            ensure!(
                (0..=255).contains(&g),
                "Invalid value for green in rgb function"
            );
            g as u8
        }
        ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(g))) => {
            ensure!(
                (0.0..=255.0).contains(&g),
                "Invalid value for green in rgb function"
            );
            g.round() as u8
        }
        _ => bail!("Invalid rgb function"),
    };
    let b = match b {
        ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(b))) => {
            ensure!(
                (0..=255).contains(&b),
                "Invalid value for blue in rgb function"
            );
            b as u8
        }
        ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(b))) => {
            ensure!(
                (0.0..=255.0).contains(&b),
                "Invalid value for blue in rgb function"
            );
            b.round() as u8
        }
        _ => bail!("Invalid rgb function"),
    };

    Ok(CssValue::Color { r, g, b, a })
}

fn name_to_rgb(name: &str) -> Option<(u8, u8, u8)> {
    match name.to_ascii_lowercase().as_str() {
        "black" => Some((0, 0, 0)),
        "gray" => Some((128, 128, 128)),
        "white" => Some((255, 255, 255)),
        "red" => Some((255, 0, 0)),
        "purple" => Some((128, 0, 128)),
        "blueviolet" => Some((138, 43, 226)),
        "green" => Some((0, 128, 0)),
        "yellowgreen" => Some((154, 205, 50)),
        "yellow" => Some((255, 255, 0)),
        "blue" => Some((0, 0, 255)),
        "aqua" => Some((0, 255, 255)),
        "orange" => Some((255, 165, 0)),
        "brown" => Some((165, 42, 42)),
        _ => None,
    }
}

pub fn rgb_to_name(r: u8, g: u8, b: u8) -> Option<String> {
    match (r, g, b) {
        (0, 0, 0) => Some("black".to_string()),
        (128, 128, 128) => Some("gray".to_string()),
        (255, 255, 255) => Some("white".to_string()),
        (255, 0, 0) => Some("red".to_string()),
        (128, 0, 128) => Some("purple".to_string()),
        (138, 43, 226) => Some("blueviolet".to_string()),
        (0, 128, 0) => Some("green".to_string()),
        (154, 205, 50) => Some("yellowgreen".to_string()),
        (255, 255, 0) => Some("yellow".to_string()),
        (0, 0, 255) => Some("blue".to_string()),
        (0, 255, 255) => Some("aqua".to_string()),
        (255, 165, 0) => Some("orange".to_string()),
        (165, 42, 42) => Some("brown".to_string()),
        _ => None,
    }
}

fn hex_to_rgba(hex: &str) -> Result<(u8, u8, u8, u8)> {
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            Ok((r, g, b, 255))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            let a = u8::from_str_radix(&hex[6..8], 16)?;
            Ok((r, g, b, a))
        }
        _ => bail!("Invalid hex color: {}", hex),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::css::token::HashType;

    #[test]
    fn parse_named_color() {
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Ident(
                "currentColor".to_string()
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::Ident("currentColor".to_string())
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Ident(
                "transparent".to_string()
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::Ident("transparent".to_string())
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Ident(
                "black".to_string()
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::Ident("black".to_string())
            }
        );
    }

    #[test]
    fn parse_valid_rgb_function() {
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::Function {
                name: "rgb".to_string(),
                values: vec![
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(255))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                ]
            }])
            .unwrap(),
            ColorProp {
                value: CssValue::Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 1.0
                }
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::Function {
                name: "rgb".to_string(),
                values: vec![
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(255))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Delim('/')),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(0.5))),
                ]
            }])
            .unwrap(),
            ColorProp {
                value: CssValue::Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 0.5
                }
            }
        );

        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::Function {
                name: "rgb".to_string(),
                values: vec![
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(255))),
                    ComponentValue::PreservedToken(CssToken::Comma),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                    ComponentValue::PreservedToken(CssToken::Comma),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                    ComponentValue::PreservedToken(CssToken::Comma),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(0.5))),
                ]
            }])
            .unwrap(),
            ColorProp {
                value: CssValue::Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 0.5
                }
            }
        );

        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::Function {
                name: "rgb".to_string(),
                values: vec![
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(10.3))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(5))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(200.15))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Delim('/')),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(0.9))),
                ]
            }])
            .unwrap(),
            ColorProp {
                value: CssValue::Color {
                    r: 10,
                    g: 5,
                    b: 200,
                    a: 0.9
                }
            }
        );
    }

    #[test]
    #[should_panic]
    fn parse_invalid_rgb_function() {
        ColorProp::parse(&mut vec![ComponentValue::Function {
            name: "rgb".to_string(),
            values: vec![
                ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(255))),
                ComponentValue::PreservedToken(CssToken::Whitespace),
                ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                ComponentValue::PreservedToken(CssToken::Whitespace),
                ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                ComponentValue::PreservedToken(CssToken::Whitespace),
                ComponentValue::PreservedToken(CssToken::Delim('/')),
            ],
        }])
        .unwrap();
    }

    #[test]
    fn parse_hex() {
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Hash(
                "000000".to_string(),
                HashType::Unrestricted
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::HexColor("000000".to_string())
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Hash(
                "000".to_string(),
                HashType::Unrestricted
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::HexColor("000000".to_string())
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Hash(
                "00000000".to_string(),
                HashType::Unrestricted
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::HexColor("00000000".to_string())
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Hash(
                "0000".to_string(),
                HashType::Unrestricted
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::HexColor("00000000".to_string())
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Hash(
                "ffffff".to_string(),
                HashType::Unrestricted
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::HexColor("ffffff".to_string())
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Hash(
                "fff".to_string(),
                HashType::Unrestricted
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::HexColor("ffffff".to_string())
            }
        );

        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Hash(
                "ff0000".to_string(),
                HashType::Unrestricted
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::HexColor("ff0000".to_string())
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Hash(
                "f00".to_string(),
                HashType::Unrestricted
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::HexColor("ff0000".to_string())
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Hash(
                "ff000000".to_string(),
                HashType::Unrestricted
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::HexColor("ff000000".to_string())
            }
        );
        assert_eq!(
            ColorProp::parse(&mut vec![ComponentValue::PreservedToken(CssToken::Hash(
                "f000".to_string(),
                HashType::Unrestricted
            ))])
            .unwrap(),
            ColorProp {
                value: CssValue::HexColor("ff000000".to_string())
            }
        );
    }
}
