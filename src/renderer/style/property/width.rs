use std::fmt;

use anyhow::{Result, bail};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::token::CssToken;
use crate::renderer::style::SpecifiedStyle;
use crate::renderer::style::property::font_size::{self, FontSizeProp};
use crate::renderer::style::property::{
    AbsoluteLengthUnit, CssProperty, CssValue, LengthUnit, RelativeLengthUnit,
    parse_length_percentage_type,
};

/// https://developer.mozilla.org/en-US/docs/Web/CSS/width
#[derive(Clone, Debug, PartialEq)]
pub struct WidthProp {
    pub size: CssValue,
}

impl fmt::Display for WidthProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.size)
    }
}

impl Default for WidthProp {
    fn default() -> Self {
        Self {
            size: CssValue::Ident("auto".to_string()),
        }
    }
}

impl CssProperty for WidthProp {
    // width =
    //   auto                                      |
    //   <length-percentage [0,∞]>                 |
    //   min-content                               |
    //   max-content                               |
    //   fit-content( <length-percentage [0,∞]> )  |
    //   <calc-size()>                             |
    //   <anchor-size()>
    fn parse(values: &[ComponentValue]) -> Result<Self> {
        let mut values = values.iter().cloned().peekable();
        // todo: implement the rest of the values
        if let Some(ComponentValue::PreservedToken(CssToken::Ident(size))) = values.peek() {
            match size.as_str() {
                "auto" => Ok(Self {
                    size: CssValue::Ident(size.to_string()),
                }),
                _ => unimplemented!(),
            }
        } else {
            Ok(Self {
                size: parse_length_percentage_type(&mut values)?,
            })
        }
    }

    fn compute(&mut self, current_style: Option<&SpecifiedStyle>) -> Result<&Self> {
        let current_font_size = current_style.and_then(|s| s.font_size.as_ref());
        let current_font_size = match current_font_size {
            Some(FontSizeProp {
                size: CssValue::Length(size, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            }) => size,
            None => &font_size::MEDIUM,
            _ => bail!("Invalid font-size value: {:?}", current_font_size),
        };
        match &self.size {
            CssValue::Ident(v) => {
                if v != "auto" {
                    unimplemented!()
                }
            }
            CssValue::Length(size, unit) => match unit {
                LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px) => {
                    self.size = CssValue::Length(
                        *size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    );
                }
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Em) => {
                    self.size = CssValue::Length(
                        size * current_font_size,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    );
                }
                _ => unimplemented!(),
            },
            CssValue::Percentage(_) => {}
            _ => unimplemented!(),
        }

        Ok(self)
    }
}
