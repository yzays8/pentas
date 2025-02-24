mod rect;
mod text;

pub use rect::RenderRect;
pub use text::RenderText;

#[derive(Debug, Clone, PartialEq)]
pub enum RenderObject {
    Text(RenderText),
    Rect(RenderRect),
}
