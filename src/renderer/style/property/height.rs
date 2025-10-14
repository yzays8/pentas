use std::fmt;

use anyhow::{Result, bail};

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

/// https://developer.mozilla.org/en-US/docs/Web/CSS/height
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

impl CssProperty for HeightProp {
    // height =
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

    fn compute(
        &mut self,
        current_style: Option<&SpecifiedStyle>,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<&Self> {
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
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Vw) => {
                    self.size = CssValue::Length(
                        size * (viewport_width as f32) / 100.0,
                        LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px),
                    );
                }
                LengthUnit::RelativeLengthUnit(RelativeLengthUnit::Vh) => {
                    self.size = CssValue::Length(
                        size * (viewport_height as f32) / 100.0,
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
