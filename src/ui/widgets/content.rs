use std::vec;

use gtk4::glib;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::net::http::HttpClient;
use crate::renderer::{RenderObject, Renderer};

mod imp {
    use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gtk4::glib::Properties;
    use gtk4::prelude::*;
    use gtk4::subclass::prelude::*;
    use gtk4::{cairo, glib, CompositeTemplate};

    use super::RenderObject;
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
                        size,
                        color,
                    } => {
                        ctx.set_source_rgb(color.0, color.1, color.2);
                        ctx.set_font_size(*size);
                        let lines = text.split("\n");
                        for (i, line) in lines.enumerate() {
                            ctx.move_to(*x, *y + size * i as f64);
                            let _ = ctx.show_text(line);
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
    pub fn on_toolbar_entry_activated(&self, query: &str) {
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

        *self.imp().objects.borrow_mut() = Renderer::run(&html, false).unwrap();

        self.imp()
            .history
            .borrow_mut()
            .add(query.to_string(), self.imp().objects.borrow().clone());

        // This is inaccurate behaviour.
        self.set_current_history_index(self.imp().history.borrow().entries.len() as i32 - 1);
        self.imp().present();
    }

    pub fn on_backward_button_clicked(&self) {
        let index = self.current_history_index() as usize;
        if index > 0 {
            self.set_current_history_index(index as i32 - 1);
            self.imp().clear();
            *self.imp().objects.borrow_mut() = self
                .imp()
                .history
                .borrow()
                .get(index - 1)
                .unwrap()
                .objects
                .clone();
            self.imp().present();
        }
    }

    pub fn on_forward_button_clicked(&self) {
        let index = self.current_history_index() as usize;
        if index < self.imp().history.borrow().entries.len() - 1 {
            self.set_current_history_index(index as i32 + 1);
            self.imp().clear();
            *self.imp().objects.borrow_mut() = self
                .imp()
                .history
                .borrow()
                .get(index + 1)
                .unwrap()
                .objects
                .clone();
            self.imp().present();
        }
    }
}
