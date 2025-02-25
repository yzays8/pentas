use gtk4::{cairo, pango};

use crate::renderer::RenderObject;

#[derive(Debug, Default)]
pub struct Painter {
    pango_ctx: pango::Context,
}

impl Painter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ctx(pango_ctx: &pango::Context) -> Self {
        Self {
            pango_ctx: pango_ctx.clone(),
        }
    }

    pub fn set_ctx(&mut self, pango_ctx: &pango::Context) {
        self.pango_ctx = pango_ctx.clone();
    }

    pub fn paint(&self, cairo_ctx: &cairo::Context, objects: &[RenderObject]) {
        for object in objects.iter() {
            match object {
                RenderObject::Text(t) => {
                    t.paint(cairo_ctx, &self.pango_ctx);
                }
                RenderObject::Rect(r) => {
                    r.paint(cairo_ctx);
                }
            }
        }
    }
}
