use std::fmt;

use anyhow::{Ok, Result};

use crate::renderer::{
    css::cssom::ComponentValue,
    layout::Edge,
    style::{
        SpecifiedStyle,
        property::{AbsoluteLengthUnit, CssProperty, CssValue, LengthUnit, color::ColorProp},
    },
};

// The values of these properties are not clearly defined in the CSS specification.
// const THIN: f32 = 1.0;
// const MEDIUM: f32 = 3.0;
// const THICK: f32 = 5.0;

// todo: Add BorderColorProp for border-color
/// https://developer.mozilla.org/en-US/docs/Web/CSS/border
#[derive(Clone, Debug)]
pub struct BorderProp {
    pub border_color: ColorProp,
    pub border_style: BorderStyleProp,
    pub border_width: BorderWidthProp,
}

impl fmt::Display for BorderProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl CssProperty for BorderProp {
    // todo: proper implementation
    #[allow(unused_variables)]
    fn parse(values: &[ComponentValue]) -> Result<Self> {
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
    fn compute(&mut self, current_style: Option<&SpecifiedStyle>) -> Result<&Self> {
        self.border_color.compute(current_style)?;
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

impl fmt::Display for BorderStyleProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl fmt::Display for BorderWidthProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl BorderWidthProp {
    pub fn to_px(&self) -> Result<Edge> {
        Ok(Edge {
            top: self.top.to_px()?,
            right: self.right.to_px()?,
            bottom: self.bottom.to_px()?,
            left: self.left.to_px()?,
        })
    }
}
