mod rect;
mod text;

pub use self::{rect::RenderRect, text::RenderText};

use gtk4::{cairo, pango};

#[derive(Debug, Clone, PartialEq)]
pub enum RenderObject {
    Text(RenderText),
    Rect(RenderRect),
}

impl Paintable for RenderObject {
    fn paint(&self, ctx: &RenderContext) {
        match self {
            RenderObject::Text(t) => t.paint(ctx),
            RenderObject::Rect(r) => r.paint(ctx),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderObjsInfo {
    pub objs: Vec<RenderObject>,
    pub max_width: f32,
    pub max_height: f32,
}

#[derive(Debug, Clone)]
pub struct RenderContext {
    pub gfx_ctx: cairo::Context,
    pub text_ctx: pango::Context,
}

pub trait Paintable {
    fn paint(&self, ctx: &RenderContext);
}

pub fn paint(gfx_ctx: &cairo::Context, text_ctx: &pango::Context, objs: &[impl Paintable]) {
    let gui_ctx = RenderContext {
        gfx_ctx: gfx_ctx.clone(),
        text_ctx: text_ctx.clone(),
    };
    objs.iter().for_each(|obj| {
        obj.paint(&gui_ctx);
    });
}
