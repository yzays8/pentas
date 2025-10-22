use std::f64::consts::PI;

use crate::renderer::object::{Paintable, RenderContext};

#[derive(Debug, Clone, PartialEq)]
pub struct RenderRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// RGB, [0.0, 1.0]
    pub color: (f64, f64, f64),
    /// (top-left, top-right, bottom-right, bottom-left)
    pub border_radius: (f64, f64, f64, f64),
}

impl Paintable for RenderRect {
    fn paint(&self, ctx: &RenderContext) {
        let (top_left_r, top_right_r, bottom_right_r, bottom_left_r) = (
            self.border_radius.0,
            self.border_radius.1,
            self.border_radius.2,
            self.border_radius.3,
        );

        ctx.gfx_ctx
            .set_source_rgb(self.color.0, self.color.1, self.color.2);

        if (top_left_r, top_right_r, bottom_right_r, bottom_left_r) == (0.0, 0.0, 0.0, 0.0) {
            ctx.gfx_ctx
                .rectangle(self.x, self.y, self.width, self.height);
            let _ = ctx.gfx_ctx.fill();
            return;
        }

        // The right direction of the viewport is the x-axis positive direction,
        // the bottom direction is the y-axis positive direction, and the angle
        // is calculated from the x-axis positive direction to the y-axis positive direction.

        // top-left corner
        ctx.gfx_ctx.arc(
            self.x + top_left_r,
            self.y + top_left_r,
            top_left_r,
            PI,
            1.5 * PI,
        );
        // top side
        ctx.gfx_ctx
            .line_to(self.x + self.width - top_right_r, self.y);
        // top-right corner
        ctx.gfx_ctx.arc(
            self.x + self.width - top_right_r,
            self.y + top_right_r,
            top_right_r,
            1.5 * PI,
            2.0 * PI,
        );
        // right side
        ctx.gfx_ctx
            .line_to(self.x + self.width, self.y + self.height - bottom_right_r);
        // bottom-right corner
        ctx.gfx_ctx.arc(
            self.x + self.width - bottom_right_r,
            self.y + self.height - bottom_right_r,
            bottom_right_r,
            0.0,
            0.5 * PI,
        );
        // bottom side
        ctx.gfx_ctx
            .line_to(self.x + bottom_left_r, self.y + self.height);
        // bottom-left corner
        ctx.gfx_ctx.arc(
            self.x + bottom_left_r,
            self.y + self.height - bottom_left_r,
            bottom_left_r,
            0.5 * PI,
            PI,
        );
        // left side
        ctx.gfx_ctx.line_to(self.x, self.y + top_left_r);

        ctx.gfx_ctx.close_path();
        let _ = ctx.gfx_ctx.fill();
    }
}
