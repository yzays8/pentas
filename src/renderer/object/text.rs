use gtk4::{cairo, pango};
use pangocairo::functions::show_layout;

#[derive(Debug, Clone, PartialEq)]
pub struct RenderText {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub font_family: Vec<String>,
    pub font_size: f64,
    pub font_weight: String,
    /// RGB, 0.0 to 1.0
    pub color: (f64, f64, f64),
    /// RGB, 0.0 to 1.0
    pub decoration_color: (f64, f64, f64),
    pub decoration_line: Vec<String>,
    pub decoration_style: String,
}

impl RenderText {
    pub fn paint(&self, cairo_ctx: &cairo::Context, pango_ctx: &pango::Context) {
        cairo_ctx.move_to(self.x, self.y);

        let layout = pango::Layout::new(pango_ctx);
        let attrs = pango::AttrList::new();

        // https://docs.gtk.org/Pango/struct.Color.html
        let font_color = (
            (self.color.0 * 65535.0) as u16,
            (self.color.1 * 65535.0) as u16,
            (self.color.2 * 65535.0) as u16,
        );
        let deco_color = (
            (self.decoration_color.0 * 65535.0) as u16,
            (self.decoration_color.1 * 65535.0) as u16,
            (self.decoration_color.2 * 65535.0) as u16,
        );

        attrs.insert(pango::AttrColor::new_foreground(
            font_color.0,
            font_color.1,
            font_color.2,
        ));
        if self.decoration_line.contains(&"underline".to_string()) {
            attrs.insert(pango::AttrColor::new_underline_color(
                deco_color.0,
                deco_color.1,
                deco_color.2,
            ));
            if self.decoration_style.eq_ignore_ascii_case("double") {
                attrs.insert(pango::AttrInt::new_underline(pango::Underline::Double));
            } else {
                attrs.insert(pango::AttrInt::new_underline(pango::Underline::Single));
            }
        }
        if self.decoration_line.contains(&"overline".to_string()) {
            attrs.insert(pango::AttrColor::new_overline_color(
                deco_color.0,
                deco_color.1,
                deco_color.2,
            ));
            attrs.insert(pango::AttrInt::new_overline(pango::Overline::Single));
        }
        if self.decoration_line.contains(&"line-through".to_string()) {
            attrs.insert(pango::AttrColor::new_strikethrough_color(
                deco_color.0,
                deco_color.1,
                deco_color.2,
            ));
            attrs.insert(pango::AttrInt::new_strikethrough(true));
        }

        layout.set_text(&self.text);
        layout.set_font_description(Some(&pango::FontDescription::from_string(&format!(
            "{} {} {}px",
            self.font_family.join(", "),
            self.font_weight,
            self.font_size
        ))));
        layout.set_attributes(Some(&attrs));
        show_layout(cairo_ctx, &layout);
    }
}
