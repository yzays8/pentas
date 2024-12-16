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

                // Adjust the height of the canvas for scrolling.
                if *y + layout.pixel_size().1 as f64 > canvas.height() as f64 {
                    canvas.set_height_request((*y + layout.pixel_size().1 as f64) as i32 + 5);
                }
            }
            RenderObject::Rectangle {
                x,
                y,
                width,
                height,
                color,
            } => {
                cairo_ctx.set_source_rgb(color.0, color.1, color.2);
                cairo_ctx.rectangle(*x, *y, *width, *height);
                let _ = cairo_ctx.fill();

                // Adjust the height of the canvas for scrolling.
                if *y + *height > canvas.height() as f64 {
                    canvas.set_height_request((*y + *height) as i32 + 5);
                }
            }
        }
    }
}
