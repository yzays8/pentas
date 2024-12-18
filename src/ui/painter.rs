use gtk4::prelude::*;
use gtk4::{cairo, pango, DrawingArea};
use pangocairo::functions::show_layout;

use crate::renderer::RenderObject;

pub fn paint(canvas: &DrawingArea, objects: &[RenderObject], cairo_ctx: &cairo::Context) {
    for object in objects.iter() {
        match object {
            RenderObject::Text {
                text,
                x,
                y,
                font_family,
                font_size,
                font_weight,
                color,
                decoration_color,
                decoration_line,
                decoration_style,
            } => {
                cairo_ctx.move_to(*x, *y);

                let pango_ctx = canvas.create_pango_context();
                let layout = pango::Layout::new(&pango_ctx);
                let attrs = pango::AttrList::new();

                // https://docs.gtk.org/Pango/struct.Color.html
                let font_color = (
                    (color.0 * 65535.0) as u16,
                    (color.1 * 65535.0) as u16,
                    (color.2 * 65535.0) as u16,
                );
                let deco_color = (
                    (decoration_color.0 * 65535.0) as u16,
                    (decoration_color.1 * 65535.0) as u16,
                    (decoration_color.2 * 65535.0) as u16,
                );

                attrs.insert(pango::AttrColor::new_foreground(
                    font_color.0,
                    font_color.1,
                    font_color.2,
                ));
                if decoration_line.contains(&"underline".to_string()) {
                    attrs.insert(pango::AttrColor::new_underline_color(
                        deco_color.0,
                        deco_color.1,
                        deco_color.2,
                    ));
                    if decoration_style.eq_ignore_ascii_case("double") {
                        attrs.insert(pango::AttrInt::new_underline(pango::Underline::Double));
                    } else {
                        attrs.insert(pango::AttrInt::new_underline(pango::Underline::Single));
                    }
                }
                if decoration_line.contains(&"overline".to_string()) {
                    attrs.insert(pango::AttrColor::new_overline_color(
                        deco_color.0,
                        deco_color.1,
                        deco_color.2,
                    ));
                    attrs.insert(pango::AttrInt::new_overline(pango::Overline::Single));
                }
                if decoration_line.contains(&"line-through".to_string()) {
                    attrs.insert(pango::AttrColor::new_strikethrough_color(
                        deco_color.0,
                        deco_color.1,
                        deco_color.2,
                    ));
                    attrs.insert(pango::AttrInt::new_strikethrough(true));
                }

                layout.set_text(text);
                layout.set_font_description(Some(&pango::FontDescription::from_string(&format!(
                    "{} {} {}px",
                    font_family.join(", "),
                    font_weight,
                    font_size
                ))));
                layout.set_attributes(Some(&attrs));
                show_layout(cairo_ctx, &layout);
            }
            RenderObject::Rect {
                x,
                y,
                width,
                height,
                color,
                border_radius,
            } => {
                let (top_left_r, top_right_r, bottom_right_r, bottom_left_r) = (
                    border_radius.0,
                    border_radius.1,
                    border_radius.2,
                    border_radius.3,
                );

                cairo_ctx.set_source_rgb(color.0, color.1, color.2);
                if (top_left_r, top_right_r, bottom_right_r, bottom_left_r) == (0.0, 0.0, 0.0, 0.0)
                {
                    cairo_ctx.rectangle(*x, *y, *width, *height);
                    let _ = cairo_ctx.fill();
                } else {
                    // The right direction of the viewport is the x-axis positive direction,
                    // the bottom direction is the y-axis positive direction, and the angle
                    // is calculated from the x-axis positive direction to the y-axis positive direction.

                    // top-left corner
                    cairo_ctx.arc(
                        x + top_left_r,
                        y + top_left_r,
                        top_left_r,
                        std::f64::consts::PI,
                        1.5 * std::f64::consts::PI,
                    );
                    // top side
                    cairo_ctx.line_to(x + width - top_right_r, *y);
                    // top-right corner
                    cairo_ctx.arc(
                        x + width - top_right_r,
                        y + top_right_r,
                        top_right_r,
                        1.5 * std::f64::consts::PI,
                        2.0 * std::f64::consts::PI,
                    );
                    // right side
                    cairo_ctx.line_to(x + width, y + height - bottom_right_r);
                    // bottom-right corner
                    cairo_ctx.arc(
                        x + width - bottom_right_r,
                        y + height - bottom_right_r,
                        bottom_right_r,
                        0.0,
                        0.5 * std::f64::consts::PI,
                    );
                    // bottom side
                    cairo_ctx.line_to(x + bottom_left_r, y + height);
                    // bottom-left corner
                    cairo_ctx.arc(
                        x + bottom_left_r,
                        y + height - bottom_left_r,
                        bottom_left_r,
                        0.5 * std::f64::consts::PI,
                        std::f64::consts::PI,
                    );
                    // left side
                    cairo_ctx.line_to(*x, y + top_left_r);

                    cairo_ctx.close_path();
                    let _ = cairo_ctx.fill();
                }
            }
        }
    }
}
