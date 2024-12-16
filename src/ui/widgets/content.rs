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
    use gtk4::glib::Properties;
    use gtk4::prelude::*;
    use gtk4::subclass::prelude::*;
    use gtk4::{glib, CompositeTemplate};

    use crate::app::VerbosityLevel;
    use crate::renderer::RenderObject;
    use crate::ui::history::{History, HistoryEntry};
    use crate::ui::painter::paint;

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
                #[strong]
                obj,
                move |_, ctx, _, _| {
                    paint(
                        &obj.imp().drawing_area.get(),
                        &obj.imp().objects.borrow(),
                        ctx,
                    )
                }
            ));

            // The initial history is an empty page.
            self.history.borrow_mut().add("", &[]);
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
        pub fn add_history(&self, query: &str, objects: &[RenderObject]) {
            self.history.borrow_mut().add(query, objects);
        }

        pub fn get_history(&self, index: usize) -> Option<HistoryEntry> {
            self.history.borrow().get(index).cloned()
        }

        /// Adds an object to the list of objects to be painted.
        pub fn add_object(&self, object: RenderObject) {
            self.objects.borrow_mut().push(object);
        }

        /// Replaces the list of objects to be painted with the given list.
        pub fn replace_objects(&self, objects: &[RenderObject]) {
            self.objects.replace(objects.to_owned());
        }

        /// Deletes all objects and repaints the background.
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

        /// Paints all added objects.
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

        self.imp().replace_objects(
            &get_render_objects(
                &html,
                self.imp().drawing_area.width(),
                self.imp().drawing_area.height(),
                &self.imp().drawing_area.create_pango_context(),
                *self.imp().verbosity.borrow(),
            )
            .unwrap(),
        );

        self.imp().add_history(query, &self.imp().objects.borrow());

        // This is inaccurate behaviour.
        self.set_current_history_index(self.imp().history.borrow().entries.len() as i32 - 1);
        self.emit_by_name::<()>(
            "history-updated",
            &[
                &self
                    .imp()
                    .get_history(self.current_history_index() as usize)
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
                        .get_history(self.current_history_index() as usize)
                        .unwrap()
                        .query,
                    &is_first_history,
                    &is_last_history,
                ],
            );

            self.imp().clear();
            self.imp()
                .replace_objects(&self.imp().history.borrow().get(index - 1).unwrap().objects);
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
                        .get_history(self.current_history_index() as usize)
                        .unwrap()
                        .query,
                    &is_first_history,
                    &is_last_history,
                ],
            );

            self.imp().clear();
            self.imp()
                .replace_objects(&self.imp().history.borrow().get(index + 1).unwrap().objects);
            self.imp().present();
        }
    }
}
