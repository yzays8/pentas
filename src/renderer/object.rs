mod rect;
mod text;

pub use self::{rect::RenderRect, text::RenderText};

#[derive(Debug, Clone, PartialEq)]
pub enum RenderObject {
    Text(RenderText),
    Rect(RenderRect),
}
