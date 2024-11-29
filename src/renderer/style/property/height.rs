use std::fmt;

use anyhow::{bail, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::token::CssToken;
use crate::renderer::style::property::font_size::{self, FontSizeProp};
use crate::renderer::style::value_type::{
    parse_length_percentage_type, AbsoluteLengthUnit, CssValue, LengthUnit, RelativeLengthUnit,
};

#[derive(Clone, Debug, PartialEq)]
pub struct HeightProp {
    pub size: CssValue,
}

impl fmt::Display for HeightProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.size)
    }
}

impl Default for HeightProp {
    fn default() -> Self {
        Self {
            size: CssValue::Ident("auto".to_string()),
        }
    }
}

impl HeightProp {
    // height =
    //   auto                                      |
    //   <length-percentage [0,∞]>                 |
    //   min-content                               |
    //   max-content                               |
    //   fit-content( <length-percentage [0,∞]> )  |
    //   <calc-size()>                             |
    //   <anchor-size()>
    pub fn parse(values: &[ComponentValue]) -> Result<Self> {
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

    pub fn compute(&mut self, current_font_size: Option<&FontSizeProp>) -> Result<&Self> {
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