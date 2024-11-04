use anyhow::{Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::style::property::color::ColorProp;
use crate::renderer::style::property::font_size::FontSizeProp;
use crate::renderer::style::value_type::{AbsoluteLengthUnit, CssValue, LengthUnit};

// The values of these properties are not clearly defined in the CSS specification.
// const THIN: f32 = 1.0;
// const MEDIUM: f32 = 3.0;
// const THICK: f32 = 5.0;

#[derive(Clone, Debug)]
pub struct BorderProp {
    pub border_color: ColorProp,
    pub border_style: BorderStyleProp,
    pub border_width: BorderWidthProp,
}

impl std::fmt::Display for BorderProp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.border_width, self.border_style, self.border_color
        )
    }
}

impl Default for BorderProp {
    fn default() -> Self {
        Self {
            border_color: ColorProp {
                value: CssValue::Ident("currentColor".to_string()),
            },
            border_style: BorderStyleProp::default(),
            border_width: BorderWidthProp::default(),
        }
    }
}

impl BorderProp {
    // todo: proper implementation
    #[allow(unused_variables)]
    pub fn parse(values: &[ComponentValue]) -> Result<Self> {
        Ok(Self {
            border_color: ColorProp {
                value: CssValue::Ident("currentColor".to_string()),
            },
            border_style: BorderStyleProp {
                top: CssValue::Ident("none".to_string()),
                right: CssValue::Ident("none".to_string()),
                bottom: CssValue::Ident("none".to_string()),
                left: CssValue::Ident("none".to_string()),
            },
            border_width: BorderWidthProp {
                top: CssValue::Ident("medium".to_string()),
                right: CssValue::Ident("medium".to_string()),
                bottom: CssValue::Ident("medium".to_string()),
                left: CssValue::Ident("medium".to_string()),
            },
        })
    }

    // todo: proper implementation
    #[allow(unused_variables)]
    pub fn compute(
        &mut self,
        current_font_size: Option<&FontSizeProp>,
        current_color: Option<&ColorProp>,
    ) -> Result<&Self> {
        self.border_color.compute(current_color)?;
        self.border_width = BorderWidthProp {
            top: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            right: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            bottom: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
            left: CssValue::Length(0.0, LengthUnit::AbsoluteLengthUnit(AbsoluteLengthUnit::Px)),
        };
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BorderStyleProp {
    pub top: CssValue,
    pub right: CssValue,
    pub bottom: CssValue,
    pub left: CssValue,
}

impl std::fmt::Display for BorderStyleProp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.top, self.right, self.bottom, self.left
        )
    }
}

impl Default for BorderStyleProp {
    fn default() -> Self {
        Self {
            top: CssValue::Ident("none".to_string()),
            right: CssValue::Ident("none".to_string()),
            bottom: CssValue::Ident("none".to_string()),
            left: CssValue::Ident("none".to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BorderWidthProp {
    pub top: CssValue,
    pub right: CssValue,
    pub bottom: CssValue,
    pub left: CssValue,
}

impl std::fmt::Display for BorderWidthProp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.top, self.right, self.bottom, self.left
        )
    }
}

impl Default for BorderWidthProp {
    fn default() -> Self {
        Self {
            top: CssValue::Ident("medium".to_string()),
            right: CssValue::Ident("medium".to_string()),
            bottom: CssValue::Ident("medium".to_string()),
            left: CssValue::Ident("medium".to_string()),
        }
    }
}
