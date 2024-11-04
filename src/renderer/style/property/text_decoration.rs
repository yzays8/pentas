use std::collections::HashMap;
use std::fmt;
use std::iter::Peekable;

use anyhow::{bail, Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::token::CssToken;
use crate::renderer::style::property::color::{parse_color_type, ColorProp};
use crate::renderer::style::value_type::CssValue;

// todo: add TextDecorationColor, TextDecorationLine, TextDecorationStyle structs for each member
#[derive(Clone, Debug, PartialEq)]
pub struct TextDecorationProp {
    pub color: ColorProp,
    pub line: Vec<CssValue>,
    pub style: CssValue,
}

impl fmt::Display for TextDecorationProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.color,
            self.line
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            self.style
        )
    }
}

impl TextDecorationProp {
    pub fn compute(&mut self, current_color: Option<&ColorProp>) -> Result<&Self> {
        self.color.compute(current_color)?;
        Ok(self)
    }
}

// text-decoration =
//   <'text-decoration-line'>   ||
//   <'text-decoration-style'>  ||
//   <'text-decoration-color'>
pub fn parse_text_decoration(values: &[ComponentValue]) -> Result<TextDecorationProp> {
    let mut values = values.iter().cloned().peekable();
    let mut ret = TextDecorationProp {
        color: ColorProp {
            value: CssValue::Ident("currentColor".to_string()),
        },
        line: vec![CssValue::Ident("none".to_string())],
        style: CssValue::Ident("solid".to_string()),
    };
    let mut is_color_parsed = false;
    let mut is_line_parsed = false;
    let mut is_style_parsed = false;

    while values.peek().is_some() {
        while values
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {}
        match values.peek() {
            Some(ComponentValue::PreservedToken(CssToken::Ident(ident))) => match ident.as_str() {
                "none" | "underline" | "overline" | "line-through" => {
                    if is_line_parsed {
                        bail!("text-decoration-line is already parsed");
                    }
                    ret.line = parse_text_decoration_line_type(&mut values)?;
                    is_line_parsed = true;
                }
                "solid" | "double" | "dotted" | "dashed" | "wavy" => {
                    if is_style_parsed {
                        bail!("text-decoration-style is already parsed");
                    }
                    ret.style = parse_text_decoration_style_type(&mut values)?;
                    is_style_parsed = true;
                }
                _ => {
                    if is_color_parsed {
                        bail!("text-decoration-color is already parsed");
                    }
                    ret.color = ColorProp {
                        value: parse_text_decoration_color_type(&mut values)?,
                    };
                    is_color_parsed = true;
                }
            },
            _ => {
                if is_color_parsed {
                    bail!("text-decoration-color is already parsed");
                }
                ret.color = ColorProp {
                    value: parse_text_decoration_color_type(&mut values)?,
                };
                is_color_parsed = true;
            }
        }
    }

    Ok(ret)
}

// text-decoration-color =
//   <color>
#[allow(dead_code)]
pub fn parse_text_decoration_color(values: &[ComponentValue]) -> Result<ColorProp> {
    Ok(ColorProp {
        value: parse_text_decoration_color_type(&mut values.iter().cloned().peekable())?,
    })
}

// text-decoration-style =
//   solid   |
//   double  |
//   dotted  |
//   dashed  |
//   wavy
#[allow(dead_code)]
pub fn parse_text_decoration_style(values: &[ComponentValue]) -> Result<CssValue> {
    parse_text_decoration_style_type(&mut values.iter().cloned().peekable())
}

// text-decoration-line =
//   none                                                |
//   [ underline || overline || line-through || blink ]
#[allow(dead_code)]
pub fn parse_text_decoration_line(values: &[ComponentValue]) -> Result<Vec<CssValue>> {
    parse_text_decoration_line_type(&mut values.iter().cloned().peekable())
}

// <text-decoration-line> =
//   none                                                |
//   [ underline || overline || line-through || blink ]
fn parse_text_decoration_line_type<I>(values: &mut Peekable<I>) -> Result<Vec<CssValue>>
where
    I: Iterator<Item = ComponentValue>,
{
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}
    if values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Ident(
            "none".to_string(),
        )))
        .is_some()
    {
        return Ok(vec![CssValue::Ident("none".to_string())]);
    }

    let mut ret = vec![];
    let mut lines = HashMap::new();
    lines.insert("underline", CssValue::Ident("underline".to_string()));
    lines.insert("overline", CssValue::Ident("overline".to_string()));
    lines.insert("line-through", CssValue::Ident("line-through".to_string()));
    while values.peek().is_some() {
        while values
            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
            .is_some()
        {}
        if let Some(ComponentValue::PreservedToken(CssToken::Ident(ident))) = values.peek() {
            if let Some(line) = lines.get(ident.as_str()) {
                ret.push(line.clone());
                lines.remove(ident.as_str());
                values.next();
            } else {
                if lines.len() == 3 {
                    bail!("Invalid text-decoration-line value: {:?}", ident);
                }
                break;
            }
        } else {
            break;
        }
    }

    Ok(ret)
}

// <text-decoration-style> =
//   solid   |
//   double  |
//   dotted  |
//   dashed  |
//   wavy
fn parse_text_decoration_style_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    while values
        .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
        .is_some()
    {}
    match values.next() {
        Some(ComponentValue::PreservedToken(CssToken::Ident(ident))) => match ident.as_str() {
            "solid" | "double" | "dotted" | "dashed" | "wavy" => {
                Ok(CssValue::Ident(ident.to_string()))
            }
            _ => bail!("Invalid text-decoration-style value: {:?}", ident),
        },
        _ => bail!("Invalid text-decoration-style value"),
    }
}

// <text-decoration-color> =
//   <color>
fn parse_text_decoration_color_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    parse_color_type(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::css::token::NumericType;

    #[test]
    fn parse_line() {
        let values = vec![
            ComponentValue::PreservedToken(CssToken::Ident("underline".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("overline".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("line-through".to_string())),
        ];
        assert_eq!(
            parse_text_decoration_line(&values).unwrap(),
            vec![
                CssValue::Ident("underline".to_string()),
                CssValue::Ident("overline".to_string()),
                CssValue::Ident("line-through".to_string())
            ]
        );
    }

    #[test]
    fn parse_style() {
        let values = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "dotted".to_string(),
        ))];
        assert_eq!(
            parse_text_decoration_style(&values).unwrap(),
            CssValue::Ident("dotted".to_string())
        );
    }

    #[test]
    fn parse_valid_text_decoration_prop() {
        let values = vec![
            ComponentValue::PreservedToken(CssToken::Ident("red".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("underline".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("overline".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("dotted".to_string())),
        ];
        assert_eq!(
            parse_text_decoration(&values).unwrap(),
            TextDecorationProp {
                color: ColorProp {
                    value: CssValue::Ident("red".to_string())
                },
                line: vec![
                    CssValue::Ident("underline".to_string()),
                    CssValue::Ident("overline".to_string())
                ],
                style: CssValue::Ident("dotted".to_string())
            }
        );

        let values = vec![
            ComponentValue::PreservedToken(CssToken::Ident("underline".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("overline".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::Function {
                name: "rgb".to_string(),
                values: vec![
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(255.0))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(0.0))),
                    ComponentValue::PreservedToken(CssToken::Whitespace),
                    ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(0.0))),
                ],
            },
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("dotted".to_string())),
        ];
        assert_eq!(
            parse_text_decoration(&values).unwrap(),
            TextDecorationProp {
                color: ColorProp {
                    value: CssValue::Color {
                        r: 255,
                        g: 0,
                        b: 0,
                        a: 1.0
                    }
                },
                line: vec![
                    CssValue::Ident("underline".to_string()),
                    CssValue::Ident("overline".to_string())
                ],
                style: CssValue::Ident("dotted".to_string())
            }
        );
    }

    #[test]
    #[should_panic]
    fn parse_invalid_text_decoration_prop() {
        let values = vec![
            ComponentValue::PreservedToken(CssToken::Ident("red".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("underline".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("wavy".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("overline".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("dotted".to_string())),
        ];
        parse_text_decoration(&values).unwrap();
    }
}
