use std::vec;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::app::VerbosityLevel;
use crate::net::http::HttpClient;
use crate::renderer::get_render_objects;

mod imp {
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use glib::subclass::InitializingObject;
    use gtk4::glib::subclass::Signal;
    use gtk4::prelude::*;
    use gtk4::subclass::prelude::*;
    use gtk4::{glib, CompositeTemplate};

    use crate::app::VerbosityLevel;
    use crate::history::History;
    use crate::renderer::RenderObjects;
    use crate::ui::painter::paint;

    // "/pentas" is just a prefix. See resouces.gresource.xml
    #[derive(Debug, CompositeTemplate, Default)]
    #[template(resource = "/pentas/ui/content.ui")]
    pub struct ContentArea {
        /// https://www.w3.org/TR/CSS22/visuren.html#viewport
        #[template_child]
        pub viewport: TemplateChild<gtk4::Viewport>,
        /// https://www.w3.org/TR/CSS22/intro.html#the-canvas
        #[template_child]
        pub canvas: TemplateChild<gtk4::DrawingArea>,

        pub history: RefCell<History>,
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

    impl ObjectImpl for ContentArea {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            self.canvas.get().set_draw_func(glib::clone!(
                #[strong]
                obj,
                move |_, ctx, _, _| {
                    // Adjust the width and the height of the canvas for scrolling.
                    // Note: Each time the canvas is resized, this closure is called.
                    if obj.imp().canvas.width()
                        != obj
                            .imp()
                            .history
                            .borrow()
                            .get_current()
                            .unwrap()
                            .objects
                            .max_width
                            .round() as i32
                    {
                        obj.imp().canvas.set_width_request(
                            obj.imp()
                                .history
                                .borrow()
                                .get_current()
                                .unwrap()
                                .objects
                                .max_width
                                .round() as i32,
                        );
                    }
                    if obj.imp().canvas.height()
                        != obj
                            .imp()
                            .history
                            .borrow()
                            .get_current()
                            .unwrap()
                            .objects
                            .max_height
                            .round() as i32
                            + 5
                    {
                        obj.imp().canvas.set_height_request(
                            obj.imp()
                                .history
                                .borrow()
                                .get_current()
                                .unwrap()
                                .objects
                                .max_height
                                .round() as i32
                                + 5,
                        );
                    }

                    paint(
                        &obj.imp().canvas.get(),
                        &obj.imp()
                            .history
                            .borrow()
                            .get_current()
                            .unwrap()
                            .objects
                            .list,
                        ctx,
                    )
                }
            ));

            // The initial history is a blank page.
            self.history.borrow_mut().add(
                "",
                &RenderObjects {
                    list: vec![],
                    max_width: self.canvas.width() as f32,
                    max_height: self.canvas.height() as f32,
                },
            );
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
        /// Paints all added objects.
        pub fn paint(&self) {
            self.canvas.queue_draw();
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

        let objects = get_render_objects(
            &html,
            self.imp().canvas.width(),
            self.imp().canvas.height(),
            &self.imp().canvas.create_pango_context(),
            *self.imp().verbosity.borrow(),
        )
        .unwrap();

        self.imp().history.borrow_mut().add(query, &objects);
        self.emit_by_name::<()>(
            "history-updated",
            &[
                &query.to_string(),
                &self.imp().history.borrow().is_rewindable(),
                &self.imp().history.borrow().is_forwardable(),
            ],
        );

        self.imp().paint();
    }

    pub fn on_backward_button_click(&self) {
        if self.imp().history.borrow().is_rewindable() {
            let history = self.imp().history.borrow_mut().rewind().unwrap().clone();
            self.emit_by_name::<()>(
                "history-updated",
                &[
                    &history.query,
                    &self.imp().history.borrow().is_rewindable(),
                    &self.imp().history.borrow().is_forwardable(),
                ],
            );
            self.imp().paint();
        }
    }

    pub fn on_forward_button_click(&self) {
        if self.imp().history.borrow().is_forwardable() {
            let history = self.imp().history.borrow_mut().forward().unwrap().clone();
            self.emit_by_name::<()>(
                "history-updated",
                &[
                    &history.query,
                    &self.imp().history.borrow().is_rewindable(),
                    &self.imp().history.borrow().is_forwardable(),
                ],
            );
            self.imp().paint();
        }
    }
}
