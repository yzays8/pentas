use std::fmt;

use crate::{
    error::Result,
    renderer::{
        css::cssom::ComponentValue,
        style::{
            SpecifiedStyle,
            property::{BackGroundColorProp, CssProperty},
        },
    },
};

/// https://developer.mozilla.org/en-US/docs/Web/CSS/background
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BackGroundProp {
    pub color: BackGroundColorProp,
}

impl fmt::Display for BackGroundProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.color)
    }
}

impl CssProperty for BackGroundProp {
    // background-color =
    //   <color>
    fn parse(values: &[ComponentValue]) -> Result<Self> {
        Ok(Self {
            color: BackGroundColorProp::parse(values)?,
        })
    }

    fn compute(
        &mut self,
        current_style: Option<&SpecifiedStyle>,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<&Self> {
        self.color
            .compute(current_style, viewport_width, viewport_height)?;
        Ok(self)
    }
}
