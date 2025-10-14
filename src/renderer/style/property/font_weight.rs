use std::{fmt, iter::Peekable};

use anyhow::{Ok, Result, bail};

use crate::renderer::{
    css::{
        cssom::ComponentValue,
        token::{CssToken, NumericType},
    },
    style::{
        SpecifiedStyle,
        property::{CssProperty, CssValue},
    },
};

/// https://developer.mozilla.org/en-US/docs/Web/CSS/font-weight
#[derive(Clone, Debug, PartialEq)]
pub struct FontWeightProp {
    pub weight: CssValue,
}

impl fmt::Display for FontWeightProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.weight)
    }
}

impl Default for FontWeightProp {
    fn default() -> Self {
        Self {
            weight: CssValue::Ident("normal".to_string()),
        }
    }
}

impl CssProperty for FontWeightProp {
    // font-weight =
    //   <font-weight-absolute>  |
    //   bolder                  |
    //   lighter
    fn parse(values: &[ComponentValue]) -> Result<Self> {
        let mut values = values.iter().cloned().peekable();
        if let Some(ComponentValue::PreservedToken(CssToken::Ident(weight))) = values.peek() {
            match weight.as_str() {
                "bolder" | "lighter" => Ok(Self {
                    weight: CssValue::Ident(weight.to_string()),
                }),
                "normal" | "bold" => Ok(Self {
                    weight: parse_font_weight_absolute(&mut values)?,
                }),
                _ => bail!("Expected \"bolder\" or \"lighter\" but found: {:?}", values),
            }
        } else {
            Ok(Self {
                weight: parse_font_weight_absolute(&mut values)?,
            })
        }
    }

    fn compute(&mut self, parent_style: Option<&SpecifiedStyle>, _: i32, _: i32) -> Result<&Self> {
        let parent_weight = parent_style.and_then(|s| s.font_weight.as_ref());
        let parent_weight = match parent_weight {
            Some(FontWeightProp {
                weight: CssValue::Ident(weight),
            }) => match weight.as_str() {
                "normal" => 400.0,
                "bold" => 700.0,
                _ => unreachable!(),
            },
            Some(FontWeightProp {
                weight: CssValue::Number(n),
            }) => *n,
            _ => unreachable!(),
        };

        if let CssValue::Ident(weight) = &self.weight {
            match weight.as_str() {
                "bolder" => match parent_weight {
                    0.0..350.0 => self.weight = CssValue::Number(400.0),
                    350.0..550.0 => self.weight = CssValue::Number(700.0),
                    550.0.. => self.weight = CssValue::Number(900.0),
                    _ => unreachable!(),
                },
                "lighter" => match parent_weight {
                    0.0..550.0 => self.weight = CssValue::Number(100.0),
                    550.0..750.0 => self.weight = CssValue::Number(400.0),
                    750.0.. => self.weight = CssValue::Number(700.0),
                    _ => unreachable!(),
                },
                "normal" | "bold" => {}
                _ => unreachable!(),
            }
        }

        Ok(self)
    }
}

impl FontWeightProp {
    pub fn to_name(&self) -> Result<String> {
        // https://developer.mozilla.org/en-US/docs/Web/CSS/font-weight#common_weight_name_mapping
        // https://docs.gtk.org/Pango/type_func.FontDescription.from_string.html
        let weight = match &self.weight {
            CssValue::Ident(weight) => match weight.as_str() {
                "normal" => "Regular",
                "bold" => "Bold",
                _ => bail!("Invalid font weight: {:?}", weight),
            },
            CssValue::Number(weight) => match weight {
                0.0..150.0 => "Thin",
                150.0..250.0 => "Extra Light",
                250.0..350.0 => "Light",
                350.0..450.0 => "Regular",
                450.0..550.0 => "Medium",
                550.0..650.0 => "Semi-Bold",
                650.0..750.0 => "Bold",
                750.0..850.0 => "Extra-Bold",
                850.0..950.0 => "Black",
                950.0.. => "Extra-Black",
                _ => bail!("Invalid font weight: {:?}", weight),
            },
            _ => bail!("Invalid font weight: {:?}", self.weight),
        };
        Ok(weight.to_string())
    }
}

// <font-weight-absolute> =
//   normal             |
//   bold               |
//   <number [1,1000]>
pub fn parse_font_weight_absolute<I>(values: &mut Peekable<I>) -> Result<CssValue>
where
    I: Iterator<Item = ComponentValue>,
{
    match values.next() {
        Some(v) => match &v {
            ComponentValue::PreservedToken(CssToken::Ident(size)) => match size.as_str() {
                "normal" | "bold" => Ok(CssValue::Ident(size.to_string())),
                _ => bail!("Expected \"normal\" or \"bold\" but found: {:?}", v),
            },
            ComponentValue::PreservedToken(CssToken::Number(NumericType::Number(n)))
                if *n >= 1.0 && *n <= 1000.0 =>
            {
                Ok(CssValue::Number(*n))
            }
            _ => bail!("Expected number between 1 and 1000 but found: {:?}", v),
        },
        None => bail!("Expected <font-weight-absolute> but found none"),
    }
}
