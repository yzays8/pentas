pub mod border;
pub mod color;
pub mod display;
pub mod font_size;
pub mod height;
pub mod margin;
pub mod padding;
pub mod text_decoration;
pub mod width;

pub use border::BorderProp;
pub use color::ColorProp;
pub use display::{DisplayBox, DisplayOutside, DisplayProp};
pub use font_size::FontSizeProp;
pub use height::HeightProp;
pub use margin::{MarginBlockProp, MarginProp};
pub use padding::PaddingProp;
pub use text_decoration::TextDecorationProp;
pub use width::WidthProp;
