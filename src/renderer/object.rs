mod rect;
mod text;

pub use self::{rect::RenderRect, text::RenderText};

#[derive(Debug, Clone, PartialEq)]
pub enum RenderObject {
    Text(RenderText),
    Rect(RenderRect),
}

#[derive(Debug, Clone, Default)]
pub struct RenderObjectsInfo {
    pub objects: Vec<RenderObject>,
    pub max_width: f32,
    pub max_height: f32,
}
