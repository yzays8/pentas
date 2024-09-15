use anyhow::{Ok, Result};

use crate::renderer::css::cssom::ComponentValue;
use crate::renderer::css::property::font_size::FontSizePx;

// The values of these properties are not clearly defined in the CSS specification.
// const THIN: f32 = 1.0;
const MEDIUM: f32 = 3.0;
// const THICK: f32 = 5.0;

#[derive(Clone, Debug)]
pub struct Border {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Default for Border {
    fn default() -> Border {
        Border {
            top: MEDIUM,
            right: MEDIUM,
            bottom: MEDIUM,
            left: MEDIUM,
        }
    }
}

impl std::fmt::Display for Border {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.top, self.right, self.bottom, self.left
        )
    }
}

impl Border {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    // todo
    #[allow(unused_variables)]
    pub fn parse(value: &[ComponentValue], parent_px: Option<FontSizePx>) -> Result<Self> {
        Ok(Self::new(0.0, 0.0, 0.0, 0.0))
    }
}
