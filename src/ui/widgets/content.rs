use std::vec;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::app::VerbosityLevel;
use crate::net::http::HttpClient;
use crate::renderer::Renderer;

mod imp {
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use glib::subclass::InitializingObject;
    use gtk4::glib::subclass::Signal;
    use gtk4::glib::Properties;
    use gtk4::prelude::*;
    use gtk4::subclass::prelude::*;
    use gtk4::{cairo, glib, pango, CompositeTemplate};
    use pangocairo;

    use crate::app::VerbosityLevel;
    use crate::renderer::RenderObject;
    use crate::ui::history::History;

    // "/pentas" is just a prefix. See resouces.gresource.xml
    #[derive(Debug, CompositeTemplate, Default, Properties)]
    #[template(resource = "/pentas/ui/content.ui")]
    #[properties(wrapper_type = super::ContentArea)]
    pub struct ContentArea {
        #[template_child]
        pub drawing_area: TemplateChild<gtk4::DrawingArea>,
        pub objects: RefCell<Vec<RenderObject>>,
        pub history: RefCell<History>,
        #[property(get, set)]
        current_history_index: RefCell<i32>,
        pub verbosity: RefCell<VerbosityLevel>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContentArea {
        const NAME: &'static str = "ContentArea";
        type Type = super::ContentArea;
        type ParentType = gtk4::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ContentArea {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            self.drawing_area.get().set_draw_func(glib::clone!(
                #[weak]
                obj,
                move |_, ctx, _, _| {
                    obj.imp().draw(ctx);
                }
            ));

            // The initial history is an empty page.
            self.history.borrow_mut().add("".to_string(), vec![]);
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("history-updated")
                    .param_types([glib::Type::STRING, glib::Type::BOOL, glib::Type::BOOL])
                    .build()]
            })
        }
    }

    impl WidgetImpl for ContentArea {}
    impl BoxImpl for ContentArea {}

    impl ContentArea {
        pub fn add_object(&self, object: RenderObject) {
            self.objects.borrow_mut().push(object);
        }

        pub fn draw(&self, ctx: &cairo::Context) {
            for object in self.objects.borrow().iter() {
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
                        ctx.move_to(*x, *y);
                        let pango_ctx = self.drawing_area.get().create_pango_context();
                        let layout = pango::Layout::new(&pango_ctx);
                        let attrs = pango::AttrList::new();
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
                                attrs.insert(pango::AttrInt::new_underline(
                                    pango::Underline::Double,
                                ));
                            } else {
                                attrs.insert(pango::AttrInt::new_underline(
                                    pango::Underline::Single,
                                ));
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
                        layout.set_font_description(Some(&pango::FontDescription::from_string(
                            &format!("{} {} {}px", font_family.join(", "), font_weight, font_size),
                        )));
                        layout.set_attributes(Some(&attrs));
                        pangocairo::functions::show_layout(ctx, &layout);

                        // Adjust the height of the drawing area for scrolling.
                        if *y + layout.pixel_size().1 as f64 > self.drawing_area.height() as f64 {
                            self.drawing_area
                                .set_height_request((*y + layout.pixel_size().1 as f64) as i32 + 5);
                        }
                    }
                    RenderObject::Rectangle {
                        x,
                        y,
                        width,
                        height,
                        color,
                    } => {
                        ctx.set_source_rgb(color.0, color.1, color.2);
                        ctx.rectangle(*x, *y, *width, *height);
                        let _ = ctx.fill();

                        // Adjust the height of the drawing area for scrolling.
                        if *y + *height > self.drawing_area.height() as f64 {
                            self.drawing_area
                                .set_height_request((*y + *height) as i32 + 5);
                        }
                    }
                }
            }
        }

        pub fn clear(&self) {
            self.objects.borrow_mut().clear();
            self.objects.borrow_mut().push(RenderObject::Rectangle {
                x: 0.0,
                y: 0.0,
                width: self.drawing_area.width() as f64,
                height: self.drawing_area.height() as f64,
                color: (1.0, 1.0, 1.0),
            });
            self.drawing_area.queue_draw();
            self.objects.borrow_mut().clear();
            self.drawing_area.set_height_request(-1);
        }

        pub fn present(&self) {
            self.drawing_area.queue_draw();
        }
    }
}

glib::wrapper! {
    pub struct ContentArea(ObjectSubclass<imp::ContentArea>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl ContentArea {
    pub fn set_verbosity(&self, verbosity: VerbosityLevel) {
        self.imp().verbosity.replace(verbosity);
    }

    pub fn on_toolbar_entry_activate(&self, query: &str) {
        self.imp().clear();

        // todo: Add a proper URL parser.
        let url = query
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let mut url = url.split('/');
        let (host, port) = if let Some(hp) = url.by_ref().next() {
            let hp = hp.split(':').collect::<Vec<&str>>();
            let host = hp.first().unwrap();
            if let Some(port) = hp.get(1) {
                if let Ok(port) = port.parse::<u16>() {
                    (*host, port)
                } else {
                    return;
                }
            } else {
                (*host, 80)
            }
        } else {
            return;
        };
        let path = if let Some(path) = url.by_ref().next() {
            format!("/{}", path)
        } else {
            "/".to_string()
        };

        let client = HttpClient::new(host, port);
        let headers = vec![
            // HTTP/1.1 client must contain Host header.
            // https://datatracker.ietf.org/doc/html/rfc9112#section-3.2
            ("Host", host),
            // ("User-Agent", "pentas"),
            // todo: Remove this header and handle Content-Length in the client.
            ("Connection", "close"),
        ];
        let html = match client.send_request("GET", &path, &headers, None) {
            Ok(response) => response.body,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };

        self.imp().objects.replace(
            Renderer::run(
                &html,
                self.imp().drawing_area.width(),
                self.imp().drawing_area.height(),
                *self.imp().verbosity.borrow(),
            )
            .unwrap(),
        );

        self.imp()
            .history
            .borrow_mut()
            .add(query.to_string(), self.imp().objects.borrow().clone());

        // This is inaccurate behaviour.
        self.set_current_history_index(self.imp().history.borrow().entries.len() as i32 - 1);
        self.emit_by_name::<()>(
            "history-updated",
            &[
                &self
                    .imp()
                    .history
                    .borrow()
                    .get(self.current_history_index() as usize)
                    .unwrap()
                    .query,
                &false,
                &true,
            ],
        );

        self.imp().present();
    }

    pub fn on_backward_button_click(&self) {
        let index = self.current_history_index() as usize;
        if index > 0 {
            self.set_current_history_index(index as i32 - 1);
            let (is_first_history, is_last_history) = (
                self.current_history_index() == 0,
                self.current_history_index()
                    == self.imp().history.borrow().entries.len() as i32 - 1,
            );
            self.emit_by_name::<()>(
                "history-updated",
                &[
                    &self
                        .imp()
                        .history
                        .borrow()
                        .get(self.current_history_index() as usize)
                        .unwrap()
                        .query,
                    &is_first_history,
                    &is_last_history,
                ],
            );

            self.imp().clear();
            self.imp().objects.replace(
                self.imp()
                    .history
                    .borrow()
                    .get(index - 1)
                    .unwrap()
                    .objects
                    .clone(),
            );
            self.imp().present();
        }
    }

    pub fn on_forward_button_click(&self) {
        let index = self.current_history_index() as usize;
        if index < self.imp().history.borrow().entries.len() - 1 {
            self.set_current_history_index(index as i32 + 1);
            let (is_first_history, is_last_history) = (
                self.current_history_index() == 0,
                self.current_history_index()
                    == self.imp().history.borrow().entries.len() as i32 - 1,
            );
            self.emit_by_name::<()>(
                "history-updated",
                &[
                    &self
                        .imp()
                        .history
                        .borrow()
                        .get(self.current_history_index() as usize)
                        .unwrap()
                        .query,
                    &is_first_history,
                    &is_last_history,
                ],
            );

            self.imp().clear();
            self.imp().objects.replace(
                self.imp()
                    .history
                    .borrow()
                    .get(index + 1)
                    .unwrap()
                    .objects
                    .clone(),
            );
            self.imp().present();
        }
    }
}
