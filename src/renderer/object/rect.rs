use gtk4::cairo;

#[derive(Debug, Clone, PartialEq)]
pub struct RenderRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// RGB, 0.0 to 1.0
    pub color: (f64, f64, f64),
    /// (top-left, top-right, bottom-right, bottom-left)
    pub border_radius: (f64, f64, f64, f64),
}

impl RenderRect {
    pub fn paint(&self, cairo_ctx: &cairo::Context) {
        let (top_left_r, top_right_r, bottom_right_r, bottom_left_r) = (
            self.border_radius.0,
            self.border_radius.1,
            self.border_radius.2,
            self.border_radius.3,
        );

        cairo_ctx.set_source_rgb(self.color.0, self.color.1, self.color.2);
        if (top_left_r, top_right_r, bottom_right_r, bottom_left_r) == (0.0, 0.0, 0.0, 0.0) {
            cairo_ctx.rectangle(self.x, self.y, self.width, self.height);
            let _ = cairo_ctx.fill();
        } else {
            // The right direction of the viewport is the x-axis positive direction,
            // the bottom direction is the y-axis positive direction, and the angle
            // is calculated from the x-axis positive direction to the y-axis positive direction.

            // top-left corner
            cairo_ctx.arc(
                self.x + top_left_r,
                self.y + top_left_r,
                top_left_r,
                std::f64::consts::PI,
                1.5 * std::f64::consts::PI,
            );
            // top side
            cairo_ctx.line_to(self.x + self.width - top_right_r, self.y);
            // top-right corner
            cairo_ctx.arc(
                self.x + self.width - top_right_r,
                self.y + top_right_r,
                top_right_r,
                1.5 * std::f64::consts::PI,
                2.0 * std::f64::consts::PI,
            );
            // right side
            cairo_ctx.line_to(self.x + self.width, self.y + self.height - bottom_right_r);
            // bottom-right corner
            cairo_ctx.arc(
                self.x + self.width - bottom_right_r,
                self.y + self.height - bottom_right_r,
                bottom_right_r,
                0.0,
                0.5 * std::f64::consts::PI,
            );
            // bottom side
            cairo_ctx.line_to(self.x + bottom_left_r, self.y + self.height);
            // bottom-left corner
            cairo_ctx.arc(
                self.x + bottom_left_r,
                self.y + self.height - bottom_left_r,
                bottom_left_r,
                0.5 * std::f64::consts::PI,
                std::f64::consts::PI,
            );
            // left side
            cairo_ctx.line_to(self.x, self.y + top_left_r);

            cairo_ctx.close_path();
            let _ = cairo_ctx.fill();
        }
    }
}
