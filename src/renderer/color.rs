use std::{fmt, panic};

use anyhow::{bail, Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::tokenizer::CssToken;
use crate::renderer::css::tokenizer::NumericType;

/// Computed value of the `color` property
#[derive(Clone, Debug, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.a == 255 {
            if let Some(name) = self.get_color_name() {
                write!(f, "{}", name)
            } else {
                write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
            }
        } else {
            write!(f, "rgba({}, {}, {}, {})", self.r, self.g, self.b, self.a)
        }
    }
}

impl Default for Color {
    fn default() -> Color {
        // Canvas text color
        Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Computes the `<color>` value, such as the `color` or `background-color` property.
    /// `currentcolor` is a keyword that represents the computed value of the `color` property,
    /// and when used for the `color` property itself, it is treated as the inherited color from its parent.
    pub fn parse(value: &[ComponentValue], current_color: Option<&Self>) -> Result<Self> {
        let value = value.first().unwrap();
        match value {
            ComponentValue::PreservedToken(t) => match t {
                CssToken::Ident(v) => {
                    let v = v.trim();
                    if v == "currentcolor" {
                        // https://developer.mozilla.org/en-US/docs/Web/CSS/color_value#currentcolor_keyword
                        if current_color.is_none() {
                            // Return the initial value of the property
                            return Ok(Color::default());
                        }
                        return Ok(current_color.unwrap().clone());
                    } else if v == "transparent" {
                        return Ok(Color::new(0, 0, 0, 0));
                    }
                    Self::parse_named_color(v)
                }
                CssToken::Hash(v, _) => {
                    let v = v.trim().to_ascii_lowercase();
                    Self::parse_hex_color(&v)
                }
                _ => bail!("Invalid color value: {:?}", value),
            },
            ComponentValue::Function { name, values } => Self::parse_color_function(name, values),
            _ => {
                bail!("Invalid color value: {:?}", value);
            }
        }
    }

    pub fn parse_named_color(value: &str) -> Result<Self> {
        match value {
            "black" => Ok(Color::new(0, 0, 0, 255)),
            "gray" => Ok(Color::new(128, 128, 128, 255)),
            "white" => Ok(Color::new(255, 255, 255, 255)),
            "red" => Ok(Color::new(255, 0, 0, 255)),
            "purple" => Ok(Color::new(128, 0, 128, 255)),
            "blueviolet" => Ok(Color::new(138, 43, 226, 255)),
            "green" => Ok(Color::new(0, 128, 0, 255)),
            "yellowgreen" => Ok(Color::new(154, 205, 50, 255)),
            "yellow" => Ok(Color::new(255, 255, 0, 255)),
            "blue" => Ok(Color::new(0, 0, 255, 255)),
            "aqua" => Ok(Color::new(0, 255, 255, 255)),
            "orange" => Ok(Color::new(255, 165, 0, 255)),
            "brown" => Ok(Color::new(165, 42, 42, 255)),
            _ => bail!("Invalid color name: {}", value),
        }
    }

    pub fn parse_hex_color(value: &str) -> Result<Self> {
        match value.len() {
            3 => {
                let r = u8::from_str_radix(&value[0..1].repeat(2), 16)?;
                let g = u8::from_str_radix(&value[1..2].repeat(2), 16)?;
                let b = u8::from_str_radix(&value[2..3].repeat(2), 16)?;
                Ok(Color::new(r, g, b, 255))
            }
            4 => {
                let r = u8::from_str_radix(&value[0..1].repeat(2), 16)?;
                let g = u8::from_str_radix(&value[1..2].repeat(2), 16)?;
                let b = u8::from_str_radix(&value[2..3].repeat(2), 16)?;
                let a = u8::from_str_radix(&value[3..4].repeat(2), 16)?;
                Ok(Color::new(r, g, b, a))
            }
            6 => {
                let r = u8::from_str_radix(&value[0..2], 16)?;
                let g = u8::from_str_radix(&value[2..4], 16)?;
                let b = u8::from_str_radix(&value[4..6], 16)?;
                Ok(Color::new(r, g, b, 255))
            }
            8 => {
                let r = u8::from_str_radix(&value[0..2], 16)?;
                let g = u8::from_str_radix(&value[2..4], 16)?;
                let b = u8::from_str_radix(&value[4..6], 16)?;
                let a = u8::from_str_radix(&value[6..8], 16)?;
                Ok(Color::new(r, g, b, a))
            }
            _ => bail!("Invalid hex color: {}", value),
        }
    }

    pub fn parse_color_function(name: &str, args: &[ComponentValue]) -> Result<Self> {
        match name {
            "rgb" => {
                let args = args
                    .iter()
                    .filter(|v| match v {
                        ComponentValue::PreservedToken(CssToken::Whitespace | CssToken::Comma) => {
                            false
                        }
                        ComponentValue::PreservedToken(
                            CssToken::Number(_) | CssToken::Delim('/'),
                        ) => true,
                        _ => panic!("Unexpected argument in rgb function"),
                    })
                    .collect::<Vec<_>>();
                let rgb = args[0..3]
                    .iter()
                    .map(|v| match v {
                        ComponentValue::PreservedToken(CssToken::Number(n)) => match n {
                            NumericType::Integer(v) => {
                                if *v < 0 || *v > 255 {
                                    panic!("Invalid value for red in rgb function");
                                }
                                *v as u8
                            }
                            NumericType::Number(v) => {
                                if *v < 0.0 || *v > 255.0 {
                                    panic!("Invalid value for red in rgb function");
                                }
                                v.round() as u8
                            }
                        },
                        _ => panic!("Unexpected argument in rgb function"),
                    })
                    .collect::<Vec<_>>();
                if args.len() == 3 {
                    Ok(Color::new(rgb[0], rgb[1], rgb[2], 255))
                } else if args.len() == 5
                    && args[3] == &ComponentValue::PreservedToken(CssToken::Delim('/'))
                {
                    let a = if let ComponentValue::PreservedToken(CssToken::Number(n)) = args[4] {
                        match n {
                            NumericType::Integer(v) => {
                                if *v < 0 || *v > 1 {
                                    panic!("Invalid value for alpha in rgb function");
                                }
                                (*v * 255) as u8
                            }
                            NumericType::Number(v) => {
                                if *v < 0.0 || *v > 1.0 {
                                    panic!("Invalid value for alpha in rgb function");
                                }
                                (v * 255.0).round() as u8
                            }
                        }
                    } else {
                        bail!("Unexpected argument in rgb function");
                    };
                    Ok(Color::new(rgb[0], rgb[1], rgb[2], a))
                } else {
                    bail!("Invalid number of arguments in rgb function");
                }
            }
            "rgba" => unimplemented!(),
            _ => unimplemented!(),
        }
    }

    pub fn get_color_name(&self) -> Option<String> {
        match self {
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            } => Some("black".to_string()),
            Color {
                r: 128,
                g: 128,
                b: 128,
                a: 255,
            } => Some("gray".to_string()),
            Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            } => Some("white".to_string()),
            Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            } => Some("red".to_string()),
            Color {
                r: 128,
                g: 0,
                b: 128,
                a: 255,
            } => Some("purple".to_string()),
            Color {
                r: 138,
                g: 43,
                b: 226,
                a: 255,
            } => Some("blueviolet".to_string()),
            Color {
                r: 0,
                g: 128,
                b: 0,
                a: 255,
            } => Some("green".to_string()),
            Color {
                r: 154,
                g: 205,
                b: 50,
                a: 255,
            } => Some("yellowgreen".to_string()),
            Color {
                r: 255,
                g: 255,
                b: 0,
                a: 255,
            } => Some("yellow".to_string()),
            Color {
                r: 0,
                g: 0,
                b: 255,
                a: 255,
            } => Some("blue".to_string()),
            Color {
                r: 0,
                g: 255,
                b: 255,
                a: 255,
            } => Some("aqua".to_string()),
            Color {
                r: 255,
                g: 165,
                b: 0,
                a: 255,
            } => Some("orange".to_string()),
            Color {
                r: 165,
                g: 42,
                b: 42,
                a: 255,
            } => Some("brown".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(
            Color::parse_hex_color("000000").unwrap(),
            Color::new(0, 0, 0, 255)
        );
        assert_eq!(
            Color::parse_hex_color("000").unwrap(),
            Color::new(0, 0, 0, 255)
        );
        assert_eq!(
            Color::parse_hex_color("00000000").unwrap(),
            Color::new(0, 0, 0, 0)
        );
        assert_eq!(
            Color::parse_hex_color("0000").unwrap(),
            Color::new(0, 0, 0, 0)
        );
        assert_eq!(
            Color::parse_hex_color("ffffff").unwrap(),
            Color::new(255, 255, 255, 255)
        );
        assert_eq!(
            Color::parse_hex_color("fff").unwrap(),
            Color::new(255, 255, 255, 255)
        );
        assert_eq!(
            Color::parse_hex_color("ff0000").unwrap(),
            Color::new(255, 0, 0, 255)
        );
        assert_eq!(
            Color::parse_hex_color("f00").unwrap(),
            Color::new(255, 0, 0, 255)
        );
        assert_eq!(
            Color::parse_hex_color("ff000000").unwrap(),
            Color::new(255, 0, 0, 0)
        );
        assert_eq!(
            Color::parse_hex_color("f000").unwrap(),
            Color::new(255, 0, 0, 0)
        );
        assert_eq!(
            Color::parse_hex_color("111").unwrap(),
            Color::new(17, 17, 17, 255)
        ); // 0x11 = 17
        assert_eq!(
            Color::parse_hex_color("1111").unwrap(),
            Color::new(17, 17, 17, 17)
        );
    }

    #[test]
    fn test_parse_function() {
        assert_eq!(
            Color::parse_color_function(
                "rgb",
                &vec![
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(255))),
                    ComponentValue::PreservedToken(CssToken::Comma),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                    ComponentValue::PreservedToken(CssToken::Comma),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                ]
            )
            .unwrap(),
            Color::new(255, 0, 0, 255)
        );
        assert_eq!(
            Color::parse_color_function(
                "rgb",
                &vec![
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(255))),
                    ComponentValue::PreservedToken(CssToken::Comma),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                    ComponentValue::PreservedToken(CssToken::Comma),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                    ComponentValue::PreservedToken(CssToken::Delim('/')),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(0))),
                ]
            )
            .unwrap(),
            Color::new(255, 0, 0, 0)
        );
        assert_eq!(
            Color::parse_color_function(
                "rgb",
                &vec![
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(255))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(15))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Integer(15))),
                    ComponentValue::PreservedToken(CssToken::Delim('/')),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(0.5))),
                ]
            )
            .unwrap(),
            Color::new(255, 15, 15, 128)
        );
    }
}
