use std::{fmt, iter::Peekable};

use anyhow::{Result, bail};

use crate::renderer::{
    css::{cssom::ComponentValue, token::CssToken},
    style::{
        SpecifiedStyle,
        property::{CssProperty, CssValue},
    },
};

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum DisplayInside {
    Flow,
    Table,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DisplayOutside {
    Block,
    Inline,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DisplayBox {
    None,
    Contents,
}

/// https://drafts.csswg.org/css-display/#the-display-properties
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayProp {
    pub inside: DisplayInside,
    pub outside: DisplayOutside,
    pub display_box: Option<DisplayBox>,
}

impl fmt::Display for DisplayProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.outside)
    }
}

impl Default for DisplayProp {
    fn default() -> Self {
        Self {
            inside: DisplayInside::Flow,
            outside: DisplayOutside::Inline,
            display_box: None,
        }
    }
}

impl CssProperty for DisplayProp {
    fn parse(values: &[ComponentValue]) -> Result<Self> {
        let mut values = values.iter().cloned().peekable();
        let mut ret = Self {
            inside: DisplayInside::Flow,
            outside: DisplayOutside::Inline,
            display_box: None,
        };

        if let Some(ComponentValue::PreservedToken(CssToken::Ident(ident))) = values.peek() {
            match ident.as_str() {
                "flow" | "table" | "block" | "inline" => {
                    let mut is_inside_parsed = false;
                    let mut is_outside_parsed = false;

                    while values.peek().is_some() {
                        while values
                            .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
                            .is_some()
                        {}
                        if let Some(ComponentValue::PreservedToken(CssToken::Ident(ident))) =
                            values.peek()
                        {
                            match ident.as_str() {
                                "flow" | "table" => {
                                    if is_inside_parsed {
                                        bail!("Inside display value is already parsed");
                                    }
                                    match parse_display_inside_type(&mut values)? {
                                        CssValue::Ident(v) => match v.as_str() {
                                            "flow" => ret.inside = DisplayInside::Flow,
                                            "table" => ret.inside = DisplayInside::Table,
                                            _ => unimplemented!(),
                                        },
                                        _ => unreachable!(),
                                    }
                                    is_inside_parsed = true;
                                }
                                "block" | "inline" => {
                                    if is_outside_parsed {
                                        bail!("Outside display value is already parsed");
                                    }
                                    match parse_display_outside_type(&mut values)? {
                                        CssValue::Ident(v) => match v.as_str() {
                                            "block" => ret.outside = DisplayOutside::Block,
                                            "inline" => ret.outside = DisplayOutside::Inline,
                                            _ => unimplemented!(),
                                        },
                                        _ => unreachable!(),
                                    }
                                    is_outside_parsed = true;
                                }
                                _ => unimplemented!(),
                            }
                        }
                    }
                }
                "none" | "contents" => match parse_display_box_type(&mut values)? {
                    CssValue::Ident(v) => match v.as_str() {
                        "none" => ret.display_box = Some(DisplayBox::None),
                        "contents" => ret.display_box = Some(DisplayBox::Contents),
                        _ => bail!("Invalid display box value: {:?}", v),
                    },
                    _ => bail!("Invalid display box value"),
                },
                _ => unimplemented!(),
            }
        }

        Ok(ret)
    }

    fn compute(&mut self, _: Option<&SpecifiedStyle>) -> Result<&Self> {
        Ok(self)
    }
}

fn parse_display_inside_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
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
                    "flow" => Ok(CssValue::Ident("flow".to_string())),
                    "table" => Ok(CssValue::Ident("table".to_string())),
                    _ => unimplemented!(),
                }
            }
            _ => bail!("Invalid inside display value: {:?}", v),
        },
        None => bail!("Expected inside display value but found none"),
    }
}

fn parse_display_outside_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
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
                    "block" => Ok(CssValue::Ident("block".to_string())),
                    "inline" => Ok(CssValue::Ident("inline".to_string())),
                    _ => unimplemented!(),
                }
            }
            _ => bail!("Invalid outside display value: {:?}", v),
        },
        None => bail!("Expected outside display value but found none"),
    }
}

fn parse_display_box_type<I>(values: &mut Peekable<I>) -> Result<CssValue>
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
                    "none" => Ok(CssValue::Ident("none".to_string())),
                    "contents" => Ok(CssValue::Ident("contents".to_string())),
                    _ => unimplemented!(),
                }
            }
            _ => bail!("Invalid outside display value: {:?}", v),
        },
        None => bail!("Expected outside display value but found none"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_display() {
        let values = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "block".to_string(),
        ))];
        let display = DisplayProp::parse(&values).unwrap();
        assert_eq!(display.inside, DisplayInside::Flow);
        assert_eq!(display.outside, DisplayOutside::Block);
        assert_eq!(display.display_box, None);

        let values = vec![
            ComponentValue::PreservedToken(CssToken::Ident("table".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("block".to_string())),
        ];
        let display = DisplayProp::parse(&values).unwrap();
        assert_eq!(display.inside, DisplayInside::Table);
        assert_eq!(display.outside, DisplayOutside::Block);
        assert_eq!(display.display_box, None);

        let values = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "none".to_string(),
        ))];
        let display = DisplayProp::parse(&values).unwrap();
        assert_eq!(display.inside, DisplayInside::Flow);
        assert_eq!(display.outside, DisplayOutside::Inline);
        assert_eq!(display.display_box, Some(DisplayBox::None));

        let values = vec![ComponentValue::PreservedToken(CssToken::Ident(
            "contents".to_string(),
        ))];
        let display = DisplayProp::parse(&values).unwrap();
        assert_eq!(display.inside, DisplayInside::Flow);
        assert_eq!(display.outside, DisplayOutside::Inline);
        assert_eq!(display.display_box, Some(DisplayBox::Contents));
    }

    #[test]
    #[should_panic]
    fn parse_invalid_display() {
        let values = vec![
            ComponentValue::PreservedToken(CssToken::Ident("block".to_string())),
            ComponentValue::PreservedToken(CssToken::Whitespace),
            ComponentValue::PreservedToken(CssToken::Ident("inline".to_string())),
        ];
        DisplayProp::parse(&values).unwrap();
    }
}
