use std::vec;

use gtk4::{glib, prelude::*, subclass::prelude::ObjectSubclassIsExt};

use crate::{
    app::TreeTraceLevel,
    net::{Url, http::HttpClient},
};

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::subclass::InitializingObject;
    use gtk4::{CompositeTemplate, glib, glib::subclass::Signal, prelude::*, subclass::prelude::*};

    use crate::{
        history::History,
        renderer::{RenderObjectsInfo, Renderer},
        ui::painter::Painter,
    };

    // "/pentas" is just a prefix. See resources.gresource.xml
    #[derive(Debug, CompositeTemplate, Default)]
    #[template(resource = "/pentas/ui/content.ui")]
    pub struct ContentArea {
        /// https://www.w3.org/TR/CSS22/visuren.html#viewport
        #[template_child]
        pub viewport: TemplateChild<gtk4::Viewport>,
        /// https://www.w3.org/TR/CSS22/intro.html#the-canvas
        #[template_child]
        pub canvas: TemplateChild<gtk4::DrawingArea>,

        pub renderer: RefCell<Renderer>,
        pub painter: RefCell<Painter>,
        pub history: RefCell<History>,
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

            self.painter
                .borrow_mut()
                .set_ctx(&self.canvas.get().create_pango_context());

            let obj = self.obj();
            self.canvas.get().set_draw_func(glib::clone!(
                #[strong]
                obj,
                move |_, ctx, _, _| {
                    let new_page_width = obj
                        .imp()
                        .history
                        .borrow()
                        .get_current()
                        .unwrap()
                        .objs_info
                        .max_width
                        .round() as i32;
                    let new_page_height = obj
                        .imp()
                        .history
                        .borrow()
                        .get_current()
                        .unwrap()
                        .objs_info
                        .max_height
                        .round() as i32
                        + 5;

                    // Adjust the width and the height of the canvas for scrolling.
                    // Note: Each time the canvas is resized, this closure is called.
                    if obj.imp().canvas.width() != new_page_width {
                        obj.imp().canvas.set_width_request(new_page_width);
                    }
                    if obj.imp().canvas.height() != new_page_height {
                        obj.imp().canvas.set_height_request(new_page_height);
                    }

                    obj.imp().painter.borrow().paint(
                        ctx,
                        &obj.imp()
                            .history
                            .borrow()
                            .get_current()
                            .unwrap()
                            .objs_info
                            .objects,
                    )
                }
            ));

            self.renderer
                .borrow_mut()
                .set_draw_ctx(&self.canvas.get().create_pango_context());

            // The initial history is a blank page.
            self.history.borrow_mut().add(
                "",
                &RenderObjectsInfo {
                    objects: vec![],
                    title: "".to_string(),
                    max_width: self.canvas.width() as f32,
                    max_height: self.canvas.height() as f32,
                },
            );
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("history-updated")
                        .param_types([
                            glib::Type::STRING,
                            glib::Type::STRING,
                            glib::Type::BOOL,
                            glib::Type::BOOL,
                        ])
                        .build(),
                ]
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
    pub fn set_tree_trace_level(&self, tree_trace_level: TreeTraceLevel) {
        self.imp()
            .renderer
            .borrow_mut()
            .set_trace_level(tree_trace_level);
    }

    pub fn on_toolbar_entry_activate(&self, query: &str) {
        let url = match Url::from_str(query) {
            Ok(url) => url,
            Err(e) => {
                eprintln!("Invalid URL: {}", e);
                return;
            }
        };
        let host = match url.host {
            Some(host) => host,
            None => {
                eprintln!("Host is not specified");
                return;
            }
        };
        let port = match url.port {
            Some(port) => port,
            None => match url.scheme.as_str() {
                "ftp" => 21,
                "http" | "ws" => 80,
                "https" | "wss" => 443,
                _ => {
                    eprintln!("Port is not specified");
                    return;
                }
            },
        };
        let path = "/".to_string() + url.path.join("/").as_str();

        let html = match url.scheme.as_str() {
            "http" | "https" => {
                let client = HttpClient::new(&host, port);
                let headers = vec![
                    // HTTP/1.1 client must contain Host header.
                    // https://datatracker.ietf.org/doc/html/rfc9112#section-3.2
                    ("Host", host.as_str()),
                    // ("User-Agent", "pentas"),
                    // todo: Remove this header and handle Content-Length in the client.
                    ("Connection", "close"),
                ];
                match client.send_request("GET", &path, &headers, None, url.scheme == "https") {
                    Ok(response) => response.body,
                    Err(e) => {
                        eprintln!("{}", e);
                        return;
                    }
                }
            }
            _ => {
                eprintln!("Unsupported scheme: {}", url.scheme);
                return;
            }
        };

        let host_name = if url.port.is_some() {
            format!("{}:{}", host, port)
        } else {
            host.to_string()
        };

        let render_objs_info = self
            .imp()
            .renderer
            .borrow()
            .get_render_objects_info(
                &html,
                &host_name,
                self.imp().canvas.width(),
                self.imp().canvas.height(),
            )
            .unwrap();

        self.imp()
            .history
            .borrow_mut()
            .add(query, &render_objs_info);
        self.emit_by_name::<()>(
            "history-updated",
            &[
                &query.to_string(),
                &render_objs_info.title,
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
                    &history.objs_info.title,
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
                    &history.objs_info.title,
                    &self.imp().history.borrow().is_rewindable(),
                    &self.imp().history.borrow().is_forwardable(),
                ],
            );
            self.imp().paint();
        }
    }
}
